use super::{WebRequest, WebResponse};

// Send response headers and body.
// Body will not be sent if the request was a HEAD request.
// Headers will be sent as UTF-8 bytes, but you need to stay in ASCII range to
// be safe.
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

    // TODO: log any IO errors when writing the response.
    // Note that this still doesn't guarantee the client got the data.
    let _ioret = stream.write_str(&*resp);

    // Send the body unless it was a HEAD request.
    // HTTP HEAD is so retarded because you can't see error bodies.
    let mut send_body = true;
    if request.is_some() && request.unwrap().method == "head" {
        send_body = false;
    }
    if send_body {
        let _ioret = stream.write(&*response.body);
    }

}
