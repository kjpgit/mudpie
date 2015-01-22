use std::collections::HashMap;
use std::ascii::OwnedAsciiExt;
use utils;

pub struct WebRequest { 
    // Header names, verb are lowercased
    // * protocol = "http/1.0" or "http/1.1"
    // * method = verb 
    // * path = /full/path
    // * query_string = "k=v&k2=v2" or ""
    // * http_xxx = value (headers)
    pub environ: HashMap<Vec<u8>, Vec<u8>>,

    // Unicode - utf8 decoded 
    pub path: String,
}

/*
   When making a request directly to an origin server, other than a
   CONNECT or server-wide OPTIONS request (as detailed below), a client
   MUST send only the absolute path and query components of the target
   URI as the request-target.  If the target URI's path component is
   empty, the client MUST send "/" as the path within the origin-form of
   request-target.  A Host header field is also sent, as defined in
   Section 5.4.

   The asterisk-form of request-target is only used for a server-wide
   OPTIONS request .
*/

/// request_bytes: request including final \r\n\r\n
pub fn parse_request(request_bytes: &[u8]) -> WebRequest {
    let lines = utils::split_bytes_on_crlf(request_bytes);

    let request_line = lines[0];
    let request_parts = utils::split_bytes_on(request_line, b' ', 2);
    assert_eq!(request_parts.len(), 3);

    let verb = request_parts[0].to_vec().into_ascii_lowercase();
    let path = request_parts[1];
    let protocol = request_parts[2].to_vec().into_ascii_lowercase();

    if protocol != b"http/1.0" && protocol != b"http/1.1" {
        panic!("unknown protocol {:?}", protocol);
    }

    let mut environ = HashMap::<Vec<u8>, Vec<u8>>::new();
    environ.insert(b"method".to_vec(), verb.to_vec());
    environ.insert(b"protocol".to_vec(), protocol.to_vec());

    assert!(path.len() > 0);
    if verb == b"options" && path == b"*" {
        environ.insert(b"path".to_vec(), path.to_vec());
        environ.insert(b"query_string".to_vec(), b"".to_vec());
    } else {
        if path[0] != b'/' {
            panic!("absolute path required: {:?}", path);
        }
        let parts = utils::split_bytes_on(path, b'?', 1); 
        if parts.len() > 1 {
            environ.insert(b"path".to_vec(), parts[0].to_vec());
            environ.insert(b"query_string".to_vec(), parts[1].to_vec());
        } else {
            environ.insert(b"path".to_vec(), path.to_vec());
            environ.insert(b"query_string".to_vec(), b"".to_vec());
        }
    }

    // Now process the headers
    let mut first = true;
    for line in lines.iter() {
        // ignore request line.  todo: more idomatic way?
        if first {
            first = false;
            continue;
        }
        if line.len() == 0 {
            // The last part (\r\n\r\n) appears as an empty header
            continue;
        }

        // "Header: Value"
        let header_parts = utils::split_bytes_on(*line, b':', 1);
        if header_parts.len() != 2 {
            panic!("invalid header {:?}", &line);
        }
        let mut header_name = b"http_".to_vec();
        header_name.extend(header_parts[0].iter().cloned());
        let header_name = header_name.into_ascii_lowercase();
        let header_value: Vec<u8> = header_parts[1].to_vec();
        environ.insert(header_name, header_value);
    }

    let path_decoded = utils::percent_decode(environ[b"path".to_vec()].as_slice());
    

    return WebRequest {
        environ: environ,
        path: String::from_utf8_lossy(path_decoded.as_slice()).into_owned(),
    };
}

#[test]
fn test_request_1() {
    let s = b"GET /foo%20bar HTTP/1.0\r\nFoo: Bar\r\nA B C: D E F\r\n\r\n";
    let r = parse_request(s);
    assert_eq!(r.environ[b"method".to_vec()], b"get".to_vec());
    assert_eq!(r.environ[b"path".to_vec()], b"/foo%20bar".to_vec());
    assert_eq!(r.environ[b"protocol".to_vec()], b"http/1.0".to_vec());
}
