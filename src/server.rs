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

#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpResponse {
    pub fn new(status_code: u16, status_text: &str) -> Self {
        HttpResponse {
            status_code,
            status_text: status_text.to_string(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn with_body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        // Automatically set Content-Length header
        self.headers.insert("Content-Length".to_string(), body.len().to_string());
        self
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_content_type(self, content_type: &str) -> Self {
        self.with_header("Content-Type", content_type)
    }

    // Format response with proper HTTP/1.1 format and \r\n line endings
    pub fn format(&self) -> String {
        let mut response = String::new();
        
        // Status line generation (HTTP/1.1 200 OK)
        response.push_str(&format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text));
        
        // Add required headers with proper formatting
        for (key, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }
        
        // Ensure proper \r\n line endings - empty line between headers and body
        response.push_str("\r\n");
        
        // Format response body
        response.push_str(&self.body);
        
        response
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
                            _ => HttpResponse::new(405, "Method Not Allowed")
                                .with_content_type("text/plain")
                                .with_body("Method not supported"),
                        };
                        
                        let formatted_response = response.format();
                        stream.write(formatted_response.as_bytes()).unwrap();
                        stream.flush().unwrap();
                    }
                    Err(e) => {
                        println!("Error parsing HTTP request: {}", e);
                        let response = HttpResponse::new(400, "Bad Request")
                            .with_content_type("text/plain")
                            .with_body("Malformed request");
                        
                        let formatted_response = response.format();
                        stream.write(formatted_response.as_bytes()).unwrap();
                        stream.flush().unwrap();
                    }
                }
            }
            Err(e) => {
                println!("Error reading from connection: {}", e);
            }
        }
    }

    fn handle_get_request(&self, request: &HttpRequest) -> HttpResponse {
        HttpResponse::new(200, "OK")
            .with_content_type("text/plain")
            .with_body(&format!("GET request to path: {}", request.path))
    }

    fn handle_post_request(&self, request: &HttpRequest) -> HttpResponse {
        HttpResponse::new(200, "OK")
            .with_content_type("text/plain")
            .with_body(&format!("POST request to path: {} with body: {}", request.path, request.body))
    }

    fn handle_put_request(&self, request: &HttpRequest) -> HttpResponse {
        HttpResponse::new(200, "OK")
            .with_content_type("text/plain")
            .with_body(&format!("PUT request to path: {} with body: {}", request.path, request.body))
    }

    fn handle_delete_request(&self, request: &HttpRequest) -> HttpResponse {
        HttpResponse::new(200, "OK")
            .with_content_type("text/plain")
            .with_body(&format!("DELETE request to path: {}", request.path))
    }
}
