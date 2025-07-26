use std::collections::HashMap;

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

    pub fn with_chunked_encoding(self) -> Self {
        self.with_header("Transfer-Encoding", "chunked")
    }

    pub fn with_connection(self, connection_type: &str) -> Self {
        self.with_header("Connection", connection_type)
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

    // Format response with chunked transfer encoding
    pub fn format_chunked(&self) -> String {
        let mut response = String::new();
        
        // Status line generation (HTTP/1.1 200 OK)
        response.push_str(&format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text));
        
        // Add required headers with proper formatting (excluding Content-Length for chunked)
        for (key, value) in &self.headers {
            if key.to_lowercase() != "content-length" && key.to_lowercase() != "transfer-encoding" {
                response.push_str(&format!("{}: {}\r\n", key, value));
            }
        }
        
        // Add Transfer-Encoding: chunked header
        response.push_str("Transfer-Encoding: chunked\r\n");
        
        // Ensure proper \r\n line endings - empty line between headers and body
        response.push_str("\r\n");
        
        // Format body as chunks
        if !self.body.is_empty() {
            let body_bytes = self.body.as_bytes();
            response.push_str(&format!("{:X}\r\n", body_bytes.len()));
            response.push_str(&self.body);
            response.push_str("\r\n");
        }
        
        // End chunk marker
        response.push_str("0\r\n\r\n");
        
        response
    }
}
