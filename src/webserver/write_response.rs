use std;
use super::{WebRequest, WebResponse};

// Send response headers and body.
// Body will not be sent if the request was a HEAD request.
// Headers will be sent as UTF-8 bytes, but you need to stay in ASCII/Latin-1
// range to be safe.
pub fn write_response(stream: &mut Writer, 
        request: Option<&WebRequest>, 
        response: &WebResponse) {
    println!("sending response: code={}, body_length={}",
            response.code, response.body.len());

    // Respond with the max version the client requested
    let mut protocol = "HTTP/1.1";
    if request.is_some() && 
            &**request.unwrap().environ.get(b"protocol").unwrap() 
            == b"http/1.0" {
        protocol = "HTTP/1.0";
    }

    let mut resp = String::new();
    resp.push_str(&*format!("{} {} {}\r\n", 
                protocol, response.code, response.status));
    resp.push_str("Connection: close\r\n");
    resp.push_str(&*format!("Content-Length: {}\r\n", 
                response.body.len()));

    for (k, v) in response.headers.iter() {
        resp.push_str(&**k);
        resp.push_str(": ");
        resp.push_str(&**v);
        resp.push_str("\r\n");
    }
    resp.push_str("\r\n");

    // Note that success still doesn't guarantee the client got the data.
    // TODO: Rust seems to have a bug and not report an error on EPIPE.
    // Wait until std::io settles down and reproduce it.
    let ioret = stream.write_all(resp.as_bytes());
    if ioret.is_err() {
        println!("error sending response headers: {}", 
            ioret.err().unwrap());
        return;
    }

    //std::old_io::timer::sleep(std::time::duration::Duration::seconds(4));

    // Send the body unless it was a HEAD request.
    // HTTP HEAD is so retarded because you can't see error bodies.
    let mut send_body = true;
    if request.is_some() && request.unwrap().method == "head" {
        send_body = false;
    }
    if send_body {
        let _ioret = stream.write_all(&*response.body);
        if ioret.is_err() {
            println!("error sending response body: {}", 
                ioret.err().unwrap());
            return;
        } else {
            //println!("ok sending response body");
        }
    }
}
