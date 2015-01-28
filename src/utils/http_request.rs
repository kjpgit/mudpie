//! Low level parsing of an HTTP Request (path and headers)

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ascii::OwnedAsciiExt; // the magic for into_ascii_lowercase

use super::byteutils;

pub struct Request {
    pub environ: HashMap<Vec<u8>, Vec<u8>>,
    pub path: String,
    pub method: String,
}


#[derive(Show)]
#[derive(PartialEq)]
enum ParseError {
    BadRequestLine,
    BadVersion,
    InvalidAbsolutePath,
    InvalidHeaderSeparator,
    InvalidHeaderWhitespace,
}


/// Parse a HTTP 1.0/1.1 request.  Must end with \r\n\r\n, which is typically
/// what your recv() loop waits for.  (Note: this does not include the body)
///
/// request_bytes: raw request including final \r\n\r\n
pub fn parse(request_bytes: &[u8]) -> Result<Request, ParseError> {
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
    assert!(request_bytes.ends_with(b"\r\n\r\n"));

    let lines = byteutils::split_bytes_on_crlf(request_bytes);

    let request_line = lines[0];
    let request_parts = byteutils::split_bytes_on(request_line, b' ', 2);
    if request_parts.len() != 3 {
        return Err(ParseError::BadRequestLine);
    }

    let method = request_parts[0].to_vec().into_ascii_lowercase();
    let path = request_parts[1];  // NB: don't copy yet
    let protocol = request_parts[2].to_vec().into_ascii_lowercase();

    // Split doesn't coalesce spaces for us
    if method.is_empty() || path.is_empty() || protocol.is_empty() {
        return Err(ParseError::BadRequestLine);
    }

    if protocol != b"http/1.0" && protocol != b"http/1.1" {
        return Err(ParseError::BadVersion);
    }

    let mut environ = HashMap::<Vec<u8>, Vec<u8>>::new();
    environ.insert(b"method".to_vec(), method.clone());
    environ.insert(b"protocol".to_vec(), protocol.clone());

    // Parse path and query string
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
    let path_decoded = byteutils::percent_decode(
            &**environ.get(b"path").unwrap());
    let path_decoded_utf8 = String::from_utf8_lossy(
            &*path_decoded).into_owned();

    // Decode method too, to make application code simpler
    let method_utf8 = String::from_utf8_lossy(
            &*method).into_owned();

    // Now process the headers
    for line in lines.iter().skip(1) {
        if line.is_empty() {
            // The last part (\r\n\r\n) appears as an empty header
            break;
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
        nice_header_name.push_all(header_parts[0]);
        let nice_header_name = nice_header_name.into_ascii_lowercase();
        
        // Strip optional whitespace around header value
        let header_value = byteutils::strip(header_parts[1]).to_vec();

        // If a header is repeated, make the values comma separated.
        // Entry API is nice (gets around borrow checker frustration)
        match environ.entry(nice_header_name) {
            Entry::Vacant(entry) => { 
                entry.insert(header_value); 
            },
            Entry::Occupied(mut entry) => {
                (*entry.get_mut()).push_all(b",");
                (*entry.get_mut()).push_all(&*header_value);
            }
        }
    }

    return Ok(Request {
        environ: environ,
        path: path_decoded_utf8,
        method: method_utf8,
    });
}

#[cfg(test)]
fn assert_header_eq(req: &Request, header: &[u8], val: &[u8]) {
    assert_eq!(&**req.environ.get(header).unwrap(), val);
}


#[test]
fn test_request_ok() {
    let s = b"GET / HTTP/1.0\r\n\r\n";
    let r = parse(s);
    assert!(r.is_ok());

    let s = b"GET /foo%20bar HTTP/1.0\r\nFoo: Bar\r\nA B C:   D E F  \r\n\r\n";
    let r = parse(s).ok().unwrap();
    assert_header_eq(&r, b"method", b"get");
    assert_header_eq(&r, b"path", b"/foo%20bar");
    assert_header_eq(&r, b"protocol", b"http/1.0");

    assert_header_eq(&r, b"http_foo", b"Bar");
    assert_header_eq(&r, b"http_a b c", b"D E F");

    assert_eq!(r.path, "/foo bar");

    let s = b"OPTIONS * HTTP/1.1\r\n\r\n";
    let r = parse(s);
    assert!(r.is_ok());
}

#[test]
fn test_request_multi_header() {
    let s = b"GET / HTTP/1.0\r\nH: foo\r\nH: bar\r\nZ: baz\r\nH:   hello again  \r\n\r\n";
    let r = parse(s).ok().unwrap();
    assert_header_eq(&r, b"http_h", b"foo,bar,hello again");
    assert_header_eq(&r, b"http_z", b"baz");
}

#[test]
fn test_request_bad() {
    let s = b"\r\n\r\n";
    let r = parse(s);
    assert_eq!(r.err().unwrap(), ParseError::BadRequestLine);

    let s = b"GET /\r\n\r\n";
    let r = parse(s);
    assert_eq!(r.err().unwrap(), ParseError::BadRequestLine);

    let s = b"GET  HTTP/1.0\r\n\r\n";
    let r = parse(s);
    assert_eq!(r.err().unwrap(), ParseError::BadRequestLine);

    let s = b"     \r\n\r\n";
    let r = parse(s);
    assert_eq!(r.err().unwrap(), ParseError::BadRequestLine);

    let s = b"GET / HTTP/3.0\r\n\r\n";
    let r = parse(s);
    assert_eq!(r.err().unwrap(), ParseError::BadVersion);

    let s = b"GET * HTTP/1.0\r\n\r\n";
    let r = parse(s);
    assert_eq!(r.err().unwrap(), ParseError::InvalidAbsolutePath);

    let s = b"GET / HTTP/1.0\r\nABC DEF\r\n\r\n";
    let r = parse(s);
    assert_eq!(r.err().unwrap(), ParseError::InvalidHeaderSeparator);

    let s = b"GET / HTTP/1.0\r\nABC : DEF\r\n\r\n";
    let r = parse(s);
    assert_eq!(r.err().unwrap(), ParseError::InvalidHeaderWhitespace);

    let s = b"GET / HTTP/1.0\r\n ABC: DEF\r\n\r\n";
    let r = parse(s);
    assert_eq!(r.err().unwrap(), ParseError::InvalidHeaderWhitespace);
}
