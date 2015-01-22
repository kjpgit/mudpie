use std::collections::HashMap;

pub struct WebResponse {
    pub data: Vec<u8>,
    pub headers: HashMap<String, String>,
}

impl WebResponse {
    pub fn new() -> WebResponse {
        return WebResponse {
                data: Vec::new(),
                headers: HashMap::new(),
            };
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }
        
    /// Shortcut for creating a successful Unicode HTML response.
    pub fn new_html(body: String) -> WebResponse {
        let mut ret = WebResponse::new();   
        ret.set_data(body.into_bytes());
        ret.set_header("Content-Type", "text/html; charset=utf-8");
        return ret;
    }
}
