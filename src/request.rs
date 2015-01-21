use std::collections::HashMap;
use utils;

pub struct WebRequest { 
    pub path: String,
    pub verb: String,
    pub protocol: String,

    // Note: header names (keys) are all *lower cased*
    pub headers: HashMap<String, String>
}


// request_bytes: request including final \r\n\r\n
pub fn parse_request(request_bytes: &[u8]) -> WebRequest {
    let lines = utils::split_bytes_on_crlf(request_bytes);

    // "verb url/path protocol"
    let request_line = lines[0];
    let request_parts = utils::split_bytes_on(request_line, b' ', 2);
    assert_eq!(request_parts.len(), 3);

    let verb = request_parts[0];
    let path = request_parts[1];
    let protocol = request_parts[2];

    let mut headers = HashMap::<String, String>::new();
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
        assert_eq!(header_parts.len(), 2);
        let header_name = String::from_utf8_lossy(header_parts[0]).into_owned();
        let header_value = String::from_utf8_lossy(header_parts[1]).into_owned();
        headers.insert(header_name, header_value);
    }

    return WebRequest {
        path: String::from_utf8(path.to_vec()).unwrap(),
        verb: String::from_utf8(verb.to_vec()).unwrap(),
        protocol: String::from_utf8(protocol.to_vec()).unwrap(),
        headers: headers
    };
}

#[test]
fn test_request_1() {
    let s = b"GET /foo%20bar HTTP/1.0\r\nFoo: Bar\r\nA B C: D E F\r\n\r\n";
    let r = parse_request(s);
    assert_eq!(r.verb, "GET");
    assert_eq!(r.path, "/foo%20bar");
    assert_eq!(r.protocol, "HTTP/1.0");
}
