#![allow(unstable)]
extern crate mudpie;
use mudpie::{WebServer, WebRequest, WebResponse};

// Example server program
// Demonstrates use of the mudpie library


fn get_index_page(_req: &WebRequest) -> WebResponse {
    let mut page = String::new();
    page.push_str("<h1>Available Resources</h1>");
    page.push_str("<ul>");
    page.push_str("<li><a href=\"/hello\">/hello</a> Shows Request Headers");
    page.push_str("<li><a href=\"/panic\">/panic</a> Simulates a crash");
    page.push_str("</ul>");
    return WebResponse::new_html(page);
}


fn get_hello_page(req: &WebRequest) -> WebResponse {
    let mut page = String::new();
    page.push_str("<h1>Hello World!</h1>");
    page.push_str("<p>Unicode text: \u{03A6}\u{03A9}\u{20AC}\u{20AA}</p>");

    page.push_str("<h3>Request Line</h3>");
    page.push_str("<ul>");
    page.push_str(format!("<li>Verb: {}", req.verb).as_slice());
    page.push_str(format!("<li>Path: {}", req.path).as_slice());
    page.push_str(format!("<li>Protocol: {}", req.protocol).as_slice());
    page.push_str("</ul>");

    page.push_str("<h3>Request Headers</h3>");
    page.push_str("<ul>");
    let mut raw_environ = Vec::new();
    for (k, v) in req.raw_environ.iter() {
        let k = String::from_utf8_lossy(k.as_slice()).into_owned();
        let v = String::from_utf8_lossy(v.as_slice()).into_owned();
        raw_environ.push((k, v));
    }
    raw_environ.sort();
    for pair in raw_environ.iter() {
        page.push_str(format!("<li>{}: {}", pair.0, pair.1).as_slice());
    }
    page.push_str("</ul>");

    return WebResponse::new_html(page);
}


// This will automatically generate a 500 Internal Server Error
fn get_panic_page(_req: &WebRequest) -> WebResponse {
    panic!("I can't go on!");
}


fn main() {
    let mut svr = WebServer::new();

    // Setup dispatch rules
    svr.add_path("/", get_index_page);
    svr.add_path("/hello", get_hello_page);
    svr.add_path("/panic", get_panic_page);

    // Run with 10 worker threads
    svr.run("127.0.0.1", 8000, 10);
}
