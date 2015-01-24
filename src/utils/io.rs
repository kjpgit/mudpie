use std::io::{TcpListener, TcpStream};
use std::io::IoError;

use utils::byteutils;

pub fn read_until_headers_end(buffer: &mut Vec<u8>,
        stream: &mut TcpStream) -> Result<usize, IoError> 
{
    let chunk_size = 4096;
    loop { 
        // Read some more data
        let ioret = stream.push(chunk_size, buffer);
        if ioret.is_err() {
            return Err(ioret.err().unwrap());
        }

        let size = ioret.ok().unwrap();
        println!("read size {}", size);
        if size == 0 {
            continue;
        }

        // Look for \r\n\r\n, which terminates the request headers
        //println!("req_buffer {}", req_buffer.len());
        let split_pos = byteutils::memmem(buffer.as_slice(), b"\r\n\r\n");
        if split_pos.is_none() {
            continue;
        }
        return Ok(split_pos.unwrap());
    }
}


pub fn read_until_size(buffer: &mut Vec<u8>,
        stream: &mut TcpStream, size: usize) -> Result<(), IoError>
{
    let chunk_size = 4096;
    while buffer.len() < size {
        let ioret = stream.push(chunk_size, buffer);
        if ioret.is_err() {
            return Err(ioret.err().unwrap());
        }
    }
    return Ok(());
}
