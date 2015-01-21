extern crate mudpie;
use mudpie::{WebServer, WebRequest, WebResponse};

// Example server program
// Demonstrates use of the mudpie library


fn get_index_page(req: &WebRequest) -> WebResponse {
    let mut page = String::new();
    page.push_str("<h1>Index</h1>");
    page.push_str("<ul>");
    page.push_str("<li><a href=\"/hello\">/hello</a>");
    page.push_str("<li><a href=\"/panic\">/panic</a>");
    page.push_str("</ul>");
    return WebResponse::new_html(page);
}


fn get_hello_page(req: &WebRequest) -> WebResponse {
    let mut page = String::new();
    page.push_str("<h1>Hello World!</h1>");
    page.push_str("<p>Sample Paragraph</p>");
    return WebResponse::new_html(page);
}


// This will automatically generate a 500 Internal Server Error
fn get_panic_page(req: &WebRequest) -> WebResponse {
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
