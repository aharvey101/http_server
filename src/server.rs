use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpRequest {
    pub fn parse(request_data: &str) -> Result<Self, &'static str> {
        let lines: Vec<&str> = request_data.lines().collect();
        
        if lines.is_empty() {
            return Err("Empty request");
        }

        // Parse HTTP request line (method, path, version)
        let request_line_parts: Vec<&str> = lines[0].split_whitespace().collect();
        if request_line_parts.len() != 3 {
            return Err("Invalid request line");
        }

        let method = request_line_parts[0].to_string();
        let path = request_line_parts[1].to_string();
        let version = request_line_parts[2].to_string();

        // Parse HTTP headers (split by lines)
        let mut headers = HashMap::new();
        let mut header_end_index = 1;

        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.is_empty() {
                header_end_index = i;
                break;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Extract request body if present
        let body = if header_end_index + 1 < lines.len() {
            lines[header_end_index + 1..].join("\n")
        } else {
            String::new()
        };

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body,
        })
    }
}

pub struct HttpServer {
    listener: TcpListener,
}

impl HttpServer {
    pub fn new(address: &str) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(address)?;
        Ok(HttpServer { listener })
    }

    pub fn start(&self) -> Result<(), std::io::Error> {
        println!("HTTP Server listening on http://{}", self.listener.local_addr()?);

        // Implement basic connection acceptance loop
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection established!");
                    self.handle_connection(stream);
                }
                Err(e) => {
                    println!("Error accepting connection: {}", e);
                }
            }
        }
        Ok(())
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        // Read incoming data from TCP stream
        let mut buffer = [0; 1024];
        
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                println!("Received {} bytes", bytes_read);
                let request_data = String::from_utf8_lossy(&buffer[..bytes_read]);
                println!("Raw Request:\n{}", request_data);
                
                // Parse HTTP request
                match HttpRequest::parse(&request_data) {
                    Ok(request) => {
                        println!("Parsed HTTP Request:");
                        println!("  Method: {}", request.method);
                        println!("  Path: {}", request.path);
                        println!("  Version: {}", request.version);
                        println!("  Headers: {:?}", request.headers);
                        println!("  Body: {}", request.body);
                        
                        // Handle different HTTP methods
                        let response = match request.method.as_str() {
                            "GET" => self.handle_get_request(&request),
                            "POST" => self.handle_post_request(&request),
                            "PUT" => self.handle_put_request(&request),
                            "DELETE" => self.handle_delete_request(&request),
                            _ => "HTTP/1.1 405 Method Not Allowed\r\n\r\nMethod not supported".to_string(),
                        };
                        
                        stream.write(response.as_bytes()).unwrap();
                        stream.flush().unwrap();
                    }
                    Err(e) => {
                        println!("Error parsing HTTP request: {}", e);
                        let response = "HTTP/1.1 400 Bad Request\r\n\r\nMalformed request";
                        stream.write(response.as_bytes()).unwrap();
                        stream.flush().unwrap();
                    }
                }
            }
            Err(e) => {
                println!("Error reading from connection: {}", e);
            }
        }
    }

    fn handle_get_request(&self, request: &HttpRequest) -> String {
        format!("HTTP/1.1 200 OK\r\n\r\nGET request to path: {}", request.path)
    }

    fn handle_post_request(&self, request: &HttpRequest) -> String {
        format!("HTTP/1.1 200 OK\r\n\r\nPOST request to path: {} with body: {}", request.path, request.body)
    }

    fn handle_put_request(&self, request: &HttpRequest) -> String {
        format!("HTTP/1.1 200 OK\r\n\r\nPUT request to path: {} with body: {}", request.path, request.body)
    }

    fn handle_delete_request(&self, request: &HttpRequest) -> String {
        format!("HTTP/1.1 200 OK\r\n\r\nDELETE request to path: {}", request.path)
    }
}
