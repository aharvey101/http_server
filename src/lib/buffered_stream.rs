use std::net::TcpStream;
use std::io::{self, Read, Write};

pub struct BufferedStream {
    stream: TcpStream,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    read_pos: usize,
    read_end: usize,
}

impl BufferedStream {
    pub fn new(stream: TcpStream, buffer_size: usize) -> Self {
        BufferedStream {
            stream,
            read_buffer: vec![0; buffer_size],
            write_buffer: Vec::with_capacity(buffer_size),
            read_pos: 0,
            read_end: 0,
        }
    }

    pub fn read_line(&mut self) -> Result<String, io::Error> {
        let mut line = String::new();
        
        loop {
            // If we need more data in the buffer
            if self.read_pos >= self.read_end {
                self.read_pos = 0;
                self.read_end = self.stream.read(&mut self.read_buffer)?;
                
                if self.read_end == 0 {
                    break; // EOF
                }
            }

            // Look for newline in current buffer
            while self.read_pos < self.read_end {
                let byte = self.read_buffer[self.read_pos];
                self.read_pos += 1;

                if byte == b'\n' {
                    return Ok(line);
                } else if byte != b'\r' {
                    line.push(byte as char);
                }
            }
        }

        if line.is_empty() {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"))
        } else {
            Ok(line)
        }
    }

    pub fn read_request(&mut self) -> Result<String, io::Error> {
        let mut request = String::new();
        let mut content_length = 0;

        // Read headers first
        loop {
            let line = self.read_line()?;
            
            if line.is_empty() {
                break;
            }

            // Check for Content-Length header
            if line.to_lowercase().starts_with("content-length:") {
                if let Some(length_str) = line.split(':').nth(1) {
                    content_length = length_str.trim().parse().unwrap_or(0);
                }
            }

            request.push_str(&line);
            request.push_str("\r\n");
        }

        request.push_str("\r\n");
        
        // Read body if Content-Length is specified
        if content_length > 0 {
            let mut body = vec![0; content_length];
            let mut total_read = 0;
            
            while total_read < content_length {
                // Use remaining buffer data first
                let available_in_buffer = self.read_end - self.read_pos;
                let to_copy = std::cmp::min(available_in_buffer, content_length - total_read);
                
                if to_copy > 0 {
                    body[total_read..total_read + to_copy]
                        .copy_from_slice(&self.read_buffer[self.read_pos..self.read_pos + to_copy]);
                    self.read_pos += to_copy;
                    total_read += to_copy;
                }
                
                // If we need more data, read directly from stream
                if total_read < content_length {
                    let bytes_read = self.stream.read(&mut body[total_read..])?;
                    if bytes_read == 0 {
                        break; // EOF
                    }
                    total_read += bytes_read;
                }
            }
            
            let body_str = String::from_utf8_lossy(&body[..total_read]);
            request.push_str(&body_str);
        }

        Ok(request)
    }

    pub fn write_response(&mut self, response: &str) -> Result<(), io::Error> {
        self.write_buffer.extend_from_slice(response.as_bytes());
        
        // Flush if buffer is getting full (e.g., > 8KB)
        if self.write_buffer.len() > 8192 {
            self.flush()?;
        }
        
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), io::Error> {
        self.stream.write_all(&self.write_buffer)?;
        self.stream.flush()?;
        self.write_buffer.clear();
        Ok(())
    }
}
