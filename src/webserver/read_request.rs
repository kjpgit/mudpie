//! Helper module for reading a WebRequest

use std::io::IoError;
use std::io::Reader;

use super::WebRequest;
use utils;


// Possible errors from `read_request`
pub enum Error {
    ReadIoError(IoError),
    InvalidRequest,
    TooLarge,
}


// Read a full request from the client (headers and body)
// max_size: max body size
pub fn read_request(stream: &mut Reader, max_size: u64) 
        -> Result<WebRequest, Error> {
    let mut req_buffer = Vec::<u8>::with_capacity(4096);
    let iores = read_until_headers_end(&mut req_buffer, stream);
    if iores.is_err() {
        return Err(Error::ReadIoError(iores.err().unwrap()));
    }

    let request_size = iores.unwrap();
    println!("read raw request: {} bytes", request_size);

    // Try to parse it
    let req = utils::http_request::parse(req_buffer.slice_to(request_size));
    if req.is_err() {
        return Err(Error::InvalidRequest);
    }

    // Valid request.  
    let req = req.ok().unwrap();
    println!("parsed request ok: method={}, path={}", req.method, req.path);

    // See if there's a body to read too.  
    let mut body = None;
    {
        // borrowing req.environ here
        let clen = req.environ.get(b"http_content-length");
        if clen.is_some() {
            let clen = utils::byteutils::parse_u64(clen.unwrap().as_slice());
            if clen.is_none() {
                // unparseable content-length
                return Err(Error::InvalidRequest);
            }

            let clen = clen.unwrap();
            println!("body size: {} bytes", clen);

            if clen > max_size {
                return Err(Error::TooLarge);
            }

            // TODO: send 100-continue if needed

            // Start one new buffer, so we don't copy when done
            let mut body_buffer = req_buffer.slice_from(request_size).to_vec();
            let iores = read_until_size(
                    &mut body_buffer, stream, clen as usize);
            if iores.is_err() {
                return Err(Error::ReadIoError(iores.err().unwrap()));
            }
            assert!(body_buffer.len() >= clen as usize);
            body = Some(body_buffer);
        }
    }

    // Got headers and body ok
    let ret = WebRequest {
        environ: req.environ,
        path: req.path,
        method: req.method,
        body: body,
    };
    return Ok(ret);
}


// Read until \r\n\r\n, which terminates the request headers
// Note: extra data may be in the buffer.
fn read_until_headers_end(buffer: &mut Vec<u8>,
        stream: &mut Reader) -> Result<usize, IoError> 
{
    let chunk_size = 4096;
    loop { 
        // Try to read some more data
        let size = try!(stream.push(chunk_size, buffer));
        //println!("read size {}", size);
        if size == 0 {
            continue;
        }

        //println!("req_buffer {}", req_buffer.len());
        let split_pos = utils::byteutils::memmem(
                buffer.as_slice(), b"\r\n\r\n");
        if split_pos.is_none() {
            continue;
        }
        return Ok(split_pos.unwrap() + 4);
    }
}


// Read until the buffer is at least size bytes long
// Note: extra data may be in the buffer.
fn read_until_size(buffer: &mut Vec<u8>,
        stream: &mut Reader, size: usize) -> Result<(), IoError>
{
    let chunk_size = 4096;
    while buffer.len() < size {
        try!(stream.push(chunk_size, buffer));
    }
    return Ok(());
}
