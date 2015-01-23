//! Low level parsing of an HTTP Request (path and headers)

use std::collections::HashMap;
use std::ascii::OwnedAsciiExt;

use byteutils;
use super::WebRequest;


#[derive(Show)]
#[derive(PartialEq)]
enum ParseError {
    BadRequestLine,
    BadVersion,
    InvalidAbsolutePath,
    InvalidHeaderSeparator,
    InvalidHeaderWhitespace,
}



/// Parse a request.  Must end with \r\n\r\n
///
/// Currently panics on invalid request, which is actually handy for manual
///testing, / as we can verify which line triggered the error.  
/// TODO: return an error which is fine grained enough to be verified by unit
/// tests.
/// request_bytes: request including final \r\n\r\n
pub fn parse_request(request_bytes: &[u8]) -> Result<WebRequest, ParseError> {
    /*

    http://tools.ietf.org/html/rfc7230#section-5.3.1

    When making a request directly to an origin server, other than a
    CONNECT or server-wide OPTIONS request (as detailed below), a client
    MUST send only the absolute path and query components of the target
    URI as the request-target.  If the target URI's path component is
    empty, the client MUST send "/" as the path within the origin-form of
    request-target.  A Host header field is also sent, as defined in
    Section 5.4.

    The asterisk-form of request-target is only used for a server-wide
    OPTIONS request .

    http://tools.ietf.org/html/rfc7230#section-3.2.4

    Header line folding is obsolete and must be rejected.  (yay!)
        
    */
    let lines = byteutils::split_bytes_on_crlf(request_bytes);

    let request_line = lines[0];
    let request_parts = byteutils::split_bytes_on(request_line, b' ', 2);
    if request_parts.len() != 3 {
        return Err(ParseError::BadRequestLine);
    }

    let method = request_parts[0].to_vec().into_ascii_lowercase();
    let path = request_parts[1];
    let protocol = request_parts[2].to_vec().into_ascii_lowercase();

    if protocol != b"http/1.0" && protocol != b"http/1.1" {
        return Err(ParseError::BadVersion);
    }

    let mut environ = HashMap::<Vec<u8>, Vec<u8>>::new();
    environ.insert(b"method".to_vec(), method.to_vec());
    environ.insert(b"protocol".to_vec(), protocol.to_vec());

    // shouldn't be possible from split
    assert!(path.len() > 0);

    if method == b"options" && path == b"*" {
        environ.insert(b"path".to_vec(), path.to_vec());
        environ.insert(b"query_string".to_vec(), b"".to_vec());
    } else {
        if path[0] != b'/' {
            return Err(ParseError::InvalidAbsolutePath);
        }
        let parts = byteutils::split_bytes_on(path, b'?', 1); 
        if parts.len() > 1 {
            environ.insert(b"path".to_vec(), parts[0].to_vec());
            environ.insert(b"query_string".to_vec(), parts[1].to_vec());
        } else {
            environ.insert(b"path".to_vec(), path.to_vec());
            environ.insert(b"query_string".to_vec(), b"".to_vec());
        }
    }

    // Also decode path into a normalized form.
    let path_decoded = byteutils::percent_decode(environ[b"path".to_vec()].as_slice());
    let path_decoded_utf8 = String::from_utf8_lossy(
            path_decoded.as_slice()).into_owned();

    // Now process the headers
    for line in lines.iter().skip(1) {
        if line.len() == 0 {
            // The last part (\r\n\r\n) appears as an empty header
            continue;
        }

        // "Header: Value"
        let header_parts = byteutils::split_bytes_on(*line, b':', 1);
        if header_parts.len() != 2 {
            return Err(ParseError::InvalidHeaderSeparator);
        }

        // Reject obsolete header folding, or illegal space around header name,
        // per RFC 7231
        let header_name = header_parts[0];
        if byteutils::strip(header_name) != header_name {
            return Err(ParseError::InvalidHeaderWhitespace);
        }

        let mut nice_header_name = b"http_".to_vec();
        nice_header_name.extend(header_parts[0].iter().cloned());
        let nice_header_name = nice_header_name
                .into_ascii_lowercase();
        
        // strip optional whitespace around header value
        let header_value = byteutils::strip(header_parts[1]);

        environ.insert(nice_header_name, header_value.to_vec());
    }

    return Ok(WebRequest {
        environ: environ,
        path: path_decoded_utf8,
        _force_private: (),
    });
}

#[test]
fn test_request_ok() {
    let s = b"GET / HTTP/1.0\r\n\r\n";
    let r = parse_request(s);
    assert!(r.is_ok());

    let s = b"GET /foo%20bar HTTP/1.0\r\nFoo: Bar\r\nA B C:   D E F  \r\n\r\n";
    let r = parse_request(s).ok().unwrap();
    assert_eq!(r.environ[b"method".to_vec()], b"get".to_vec());
    assert_eq!(r.environ[b"path".to_vec()], b"/foo%20bar".to_vec());
    assert_eq!(r.environ[b"protocol".to_vec()], b"http/1.0".to_vec());

    assert_eq!(r.environ[b"http_foo".to_vec()].as_slice(), b"Bar");
    assert_eq!(r.environ[b"http_a b c".to_vec()].as_slice(), b"D E F");

    assert_eq!(r.path, "/foo bar");

    let s = b"OPTIONS * HTTP/1.1\r\n\r\n";
    let r = parse_request(s);
    assert!(r.is_ok());
}

#[test]
fn test_request_bad() {
    let s = b"GET /\r\n\r\n";
    let r = parse_request(s);
    assert_eq!(r.err().unwrap(), ParseError::BadRequestLine);

    let s = b"GET / HTTP/3.0\r\n\r\n";
    let r = parse_request(s);
    assert_eq!(r.err().unwrap(), ParseError::BadVersion);

    let s = b"GET * HTTP/1.0\r\n\r\n";
    let r = parse_request(s);
    assert_eq!(r.err().unwrap(), ParseError::InvalidAbsolutePath);

    let s = b"GET / HTTP/1.0\r\nABC DEF\r\n";
    let r = parse_request(s);
    assert_eq!(r.err().unwrap(), ParseError::InvalidHeaderSeparator);

    let s = b"GET / HTTP/1.0\r\nABC : DEF\r\n";
    let r = parse_request(s);
    assert_eq!(r.err().unwrap(), ParseError::InvalidHeaderWhitespace);

    let s = b"GET / HTTP/1.0\r\n ABC: DEF\r\n";
    let r = parse_request(s);
    assert_eq!(r.err().unwrap(), ParseError::InvalidHeaderWhitespace);
}
