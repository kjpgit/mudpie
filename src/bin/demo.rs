#![cfg_attr(test, allow(dead_code))]

extern crate mudpie;
use mudpie::{WebServer, WebRequest, WebResponse};
use mudpie::html_element_escape;

/*

Example server program that demonstrates use of the mudpie server

Usage: ./demo [address] [port]
Example: ./demo 0.0.0.0 8000
Optional env vars: 
    MUDPIE_THREADS=<NUM>
    MUDPIE_LOGGING=[0|1]

*/


fn main() {
    let mut args = Vec::new();
    args.extend(std::env::args());

    // Default options
    let mut listen_addr = "127.0.0.1";
    let mut listen_port = 8000;

    if args.len() > 1 {
        listen_addr = &args[1];
    }
    if args.len() > 2 {
        listen_port = match (&args[2]).parse::<i32>() {
            Ok(n) => n,
            Err(err) => panic!("invalid port number: {}", err)
        }
    }


    let mut svr = WebServer::new();
    //svr.set_max_request_body_size(10);

    // Setup dispatch rules
    svr.add_path("get", "/bench", bench_page);
    svr.add_path("GET, HEAD", "/", index_page);
    svr.add_path("get, head", "/hello", hello_page);
    svr.add_path_prefix("get,head", "/hello/", hello_page);
    svr.add_path("get", "/panic", panic_page);

    svr.add_path("get", "/form_enter", form_enter);
    svr.add_path("post", "/form_post", form_post);

    svr.add_path("put,options,foo", "/silly_methods", hello_page);

    svr.run(listen_addr, listen_port);
}


// Add html and body tags
fn to_html(input: String) -> String {
    let mut page = String::new();
    page.push_str("<html>");
    page.push_str("<body>");
    page.push_str(&input);
    page.push_str("</body>");
    page.push_str("</html>");
    return page;
}


// Return a html table of debug info
fn get_debug_info(req: &WebRequest) -> String {
    let mut page = String::new();
    page.push_str("<h2>Request Debug Info</h2>");
    page.push_str("<table>");
    let mut raw_environ = Vec::new();
    for (k, v) in req.get_environ().iter() {
        let k = String::from_utf8_lossy(&k).into_owned();
        let v = String::from_utf8_lossy(&v).into_owned();
        raw_environ.push((k, v));
    }
    raw_environ.sort();
    for pair in raw_environ.iter() {
        page.push_str("<tr>");
        page.push_str("<td>");
        page.push_str(&html_element_escape(&pair.0));
        page.push_str("<td>");
        page.push_str(&html_element_escape(&pair.1));
    }
    page.push_str("</table>");
    page.push_str("<h2>Request Body</h2>");
    let body = req.get_body();
    let body = String::from_utf8_lossy(&body).into_owned();
    page.push_str(&html_element_escape(&body));
    return page;
}


fn index_page(_req: &WebRequest) -> WebResponse {
    let mut page = String::new();
    page.push_str("<h1>Mudpie Demo Application</h1>");
    page.push_str("<h2>Example Resources</h2>");
    page.push_str(r##"
<dl>

<dt><a href="/hello?foo=bar">/hello</a> 
<dd>Hello page, shows Request Headers, outputs custom response header

<dt><a href="/hello/some/resource">/hello/some/resource</a> 
<dd>Anything under the "/hello/" prefix also works

<dt><a href="/panic">/panic</a> 
<dd>A crashing handler

<dt><a href="/form_enter">/form_enter</a> 
<dd>Form Submission Example

<dt><a href="/form_post">/form_post</a> 
<dd>Only allows POST

<dt><a href="/silly_methods">/silly_methods</a> 
<dd>Only allows PUT, OPTIONS, and FOO methods. See Allow: header

<dt><a href="/bench">/bench</a> 
<dd>A super-tiny resource useful for benchmarking socket performance

</dl>
"##);
    page = to_html(page);
    return WebResponse::new_html(page);
}


fn hello_page(req: &WebRequest) -> WebResponse {
    let mut page = String::new();
    page.push_str("<h1>Hello World!</h1>");
    page.push_str("<p>Unicode text: \u{03A6}\u{03A9}\u{20AC}\u{20AA}</p>");
    page.push_str(&get_debug_info(req));
    page = to_html(page);
    let mut ret = WebResponse::new_html(page);
    ret.set_header("x-mudpie-example-header", "fi fi fo fum");
    return ret;
}


fn bench_page(_req: &WebRequest) -> WebResponse {
    let page = "Hello World!".to_string();
    return WebResponse::new_html(page);
}


// This will automatically generate a 500 Internal Server Error
fn panic_page(_req: &WebRequest) -> WebResponse {
    panic!("I can't go on!");
}


fn form_enter(_req: &WebRequest) -> WebResponse {
    let mut page = String::new();
    page.push_str("<h1>Form Example</h1>");
    page.push_str(r##"
<h2>POST to /form_post</h2>
<form action="/form_post" method="Post">
First Name: <input type="text" name="fname">
<br/>
Last Name: <input type="text" name="lname">
<br/>
<input type="submit" value="Submit">
</form>
"##);
    page = to_html(page);
    return WebResponse::new_html(page);
}


fn form_post(req: &WebRequest) -> WebResponse {
    let mut page = String::new();
    page.push_str("<h1>Thank you for the POST</h1>");
    page.push_str(&get_debug_info(req));
    return WebResponse::new_html(page);
}
