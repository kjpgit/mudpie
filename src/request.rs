use utils::memmem;

pub struct WebRequest { 
    pub path: String,
}


// request_bytes: all headers 
pub fn parse_request(request_bytes: &[u8]) -> WebRequest {
    let proto_end = memmem(request_bytes, b"\r\n").unwrap();
    let proto_line = request_bytes.slice(0, proto_end as usize);

    println!("proto line: {}", String::from_utf8_lossy(proto_line));
    let verb_end = memmem(proto_line, b" ").unwrap();

    let verb = proto_line.slice(0, verb_end);

    let path_buffer = proto_line.slice(verb_end + 1, proto_line.len());
    let path_end = memmem(path_buffer, b" ").unwrap();
    let path = path_buffer.slice(0, path_end);

    println!("verb: {}, path: {}", 
            String::from_utf8_lossy(verb),
            String::from_utf8_lossy(path));

    return WebRequest {
        path: String::from_utf8(path.to_vec()).unwrap(),
    };
}

