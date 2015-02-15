//use std;

use super::{WebRequest, WebResponse};
use utils::genericsocket::GenericSocket;


// Send response headers and body.
// Body will not be sent if the request was a HEAD request.
// Headers will be sent as UTF-8 bytes, but you need to stay in ASCII/Latin-1
// range to be safe.
pub fn write_response(stream: &mut GenericSocket, 
        request: Option<&WebRequest>, 
        response: &WebResponse) {

    // Respond with the max version the client requested
    let mut protocol = "HTTP/1.1";
    if request.is_some() {
        let req = request.unwrap();
        if &**req.environ.get(b"protocol").unwrap() == b"http/1.0" {
            protocol = "HTTP/1.0";
        }

        // TODO: Better machine parsable output
        println!("method={} path={} code={} body_len={}",
            req.get_method(), 
            req.get_path(),
            response.code,
            response.body.len());
    } else {
        // Didn't get a valid request
        println!("code={} body_len={}",
            response.code,
            response.body.len());
    }

    let mut resp = String::new();
    resp.push_str(&format!("{} {} {}\r\n", 
                protocol, response.code, response.status));
    resp.push_str("Connection: close\r\n");
    resp.push_str(&format!("Content-Length: {}\r\n", 
                response.body.len()));

    for (k, v) in response.headers.iter() {
        resp.push_str(&k);
        resp.push_str(": ");
        resp.push_str(&v);
        resp.push_str("\r\n");
    }
    resp.push_str("\r\n");

    // Note that success still doesn't guarantee the client got the data.
    let ioret = stream.write_all(resp.as_bytes());
    if ioret.is_err() {
        return;
    }

    // Send the body unless it was a HEAD request.
    // HTTP HEAD is so retarded because you can't see error bodies.
    let mut send_body = true;
    if request.is_some() && request.unwrap().method == "head" {
        send_body = false;
    }
    if send_body {
        let ioret = stream.write_all(&response.body);
        if ioret.is_err() {
            return;
        } 
    }
}
