use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::io::{self, ErrorKind};

// Custom error types for better error handling
#[derive(Debug)]
pub enum ServerError {
    IoError(io::Error),
    ParseError(String),
    TimeoutError,
    ConnectionError(String),
    FileError(String),
}

impl From<io::Error> for ServerError {
    fn from(error: io::Error) -> Self {
        ServerError::IoError(error)
    }
}

// Logger for comprehensive logging
pub struct Logger {
    start_time: SystemTime,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            start_time: SystemTime::now(),
        }
    }

    pub fn log_info(&self, message: &str) {
        let timestamp = self.get_timestamp();
        println!("[{}] INFO: {}", timestamp, message);
    }

    pub fn log_error(&self, message: &str) {
        let timestamp = self.get_timestamp();
        eprintln!("[{}] ERROR: {}", timestamp, message);
    }

    pub fn log_warning(&self, message: &str) {
        let timestamp = self.get_timestamp();
        println!("[{}] WARNING: {}", timestamp, message);
    }

    pub fn log_request(&self, method: &str, path: &str, status: u16, client_addr: &str) {
        let timestamp = self.get_timestamp();
        println!("[{}] {} {} - {} {}", timestamp, client_addr, method, path, status);
    }

    fn get_timestamp(&self) -> String {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs();
                let hours = (secs / 3600) % 24;
                let minutes = (secs / 60) % 60;
                let seconds = secs % 60;
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            }
            Err(_) => "00:00:00".to_string(),
        }
    }
}

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

#[derive(Debug)]
pub struct Route {
    pub method: String,
    pub path: String,
    pub handler: fn(&HttpRequest) -> HttpResponse,
}

pub struct Router {
    routes: Vec<Route>,
    static_dir: Option<String>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            routes: Vec::new(),
            static_dir: None,
        }
    }

    pub fn add_route(&mut self, method: &str, path: &str, handler: fn(&HttpRequest) -> HttpResponse) {
        self.routes.push(Route {
            method: method.to_string(),
            path: path.to_string(),
            handler,
        });
    }

    pub fn set_static_dir(&mut self, dir: &str) {
        self.static_dir = Some(dir.to_string());
    }

    // Create route matching logic
    pub fn route(&self, request: &HttpRequest) -> HttpResponse {
        // Extract path without query parameters for routing
        let path_without_query = if let Some(query_start) = request.path.find('?') {
            &request.path[..query_start]
        } else {
            &request.path
        };

        // Handle different URL paths - exact match first
        for route in &self.routes {
            if route.method == request.method && route.path == path_without_query {
                return (route.handler)(request);
            }
        }

        // Handle static file serving
        if request.method == "GET" && self.static_dir.is_some() {
            if let Some(response) = self.serve_static_file(path_without_query) {
                return response;
            }
        }

        // Implement 404 Not Found responses
        HttpResponse::new(404, "Not Found")
            .with_content_type("text/html")
            .with_body("<h1>404 - Page Not Found</h1><p>The requested resource could not be found.</p>")
    }

    // Handle static file serving with enhanced error handling
    fn serve_static_file(&self, path: &str) -> Option<HttpResponse> {
        if let Some(static_dir) = &self.static_dir {
            let file_path = if path == "/" {
                format!("{}/index.html", static_dir)
            } else {
                format!("{}{}", static_dir, path)
            };

            // Security check - prevent directory traversal
            if file_path.contains("..") {
                return Some(
                    HttpResponse::new(403, "Forbidden")
                        .with_content_type("text/html")
                        .with_body("<h1>403 - Forbidden</h1><p>Directory traversal is not allowed.</p>")
                );
            }

            if Path::new(&file_path).exists() {
                match fs::read_to_string(&file_path) {
                    Ok(content) => {
                        let content_type = self.get_content_type(&file_path);
                        return Some(
                            HttpResponse::new(200, "OK")
                                .with_content_type(&content_type)
                                .with_body(&content)
                        );
                    }
                    Err(e) => {
                        // Log the specific file error
                        eprintln!("File read error for {}: {}", file_path, e);
                        return Some(
                            HttpResponse::new(500, "Internal Server Error")
                                .with_content_type("text/html")
                                .with_body("<h1>500 - Internal Server Error</h1><p>Unable to read the requested file.</p>")
                        );
                    }
                }
            }
        }
        None
    }

    // Handle different MIME types
    fn get_content_type(&self, file_path: &str) -> String {
        match file_path.split('.').last() {
            Some("html") => "text/html".to_string(),
            Some("css") => "text/css".to_string(),
            Some("js") => "application/javascript".to_string(),
            Some("json") => "application/json".to_string(),
            Some("png") => "image/png".to_string(),
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("txt") => "text/plain".to_string(),
            _ => "text/plain".to_string(),
        }
    }

    // Add support for query parameters
    pub fn parse_query_params(path: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        if let Some(query_start) = path.find('?') {
            let query_string = &path[query_start + 1..];
            for pair in query_string.split('&') {
                if let Some(eq_pos) = pair.find('=') {
                    let key = &pair[..eq_pos];
                    let value = &pair[eq_pos + 1..];
                    params.insert(key.to_string(), value.to_string());
                } else {
                    params.insert(pair.to_string(), String::new());
                }
            }
        }
        
        params
    }
}

pub struct HttpServer {
    listener: TcpListener,
    router: Router,
    logger: Logger,
}

impl HttpServer {
    pub fn new(address: &str) -> Result<Self, ServerError> {
        let listener = TcpListener::bind(address)?;
        let mut router = Router::new();
        let logger = Logger::new();
        
        // Add some default routes
        router.add_route("GET", "/", Self::handle_home);
        router.add_route("GET", "/hello", Self::handle_hello);
        router.add_route("GET", "/api/status", Self::handle_status);
        router.add_route("POST", "/api/echo", Self::handle_echo);
        
        Ok(HttpServer { listener, router, logger })
    }

    pub fn add_route(&mut self, method: &str, path: &str, handler: fn(&HttpRequest) -> HttpResponse) {
        self.router.add_route(method, path, handler);
    }

    pub fn set_static_dir(&mut self, dir: &str) {
        self.router.set_static_dir(dir);
    }

    pub fn start(&self) -> Result<(), ServerError> {
        let addr = self.listener.local_addr()?;
        self.logger.log_info(&format!("HTTP Server starting on http://{}", addr));
        
        // Set read timeout for connections to handle timeout errors
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    // Get client address for logging
                    let client_addr = stream.peer_addr()
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|_| "unknown".to_string());
                    
                    self.logger.log_info(&format!("New connection from {}", client_addr));
                    
                    // Add timeout handling for connections
                    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(30))) {
                        self.logger.log_warning(&format!("Failed to set read timeout: {}", e));
                    }
                    
                    // Handle connection with enhanced error handling
                    if let Err(e) = self.handle_connection_safe(stream, &client_addr) {
                        self.logger.log_error(&format!("Connection error for {}: {:?}", client_addr, e));
                    }
                }
                Err(e) => {
                    // Implement proper error handling for TCP operations
                    match e.kind() {
                        ErrorKind::WouldBlock | ErrorKind::TimedOut => {
                            self.logger.log_warning(&format!("Connection timeout: {}", e));
                            continue;
                        }
                        ErrorKind::ConnectionRefused | ErrorKind::ConnectionReset => {
                            self.logger.log_warning(&format!("Connection refused/reset: {}", e));
                            continue;
                        }
                        _ => {
                            self.logger.log_error(&format!("Error accepting connection: {}", e));
                            return Err(ServerError::ConnectionError(e.to_string()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // Enhanced connection handling with comprehensive error handling
    fn handle_connection_safe(&self, mut stream: TcpStream, client_addr: &str) -> Result<(), ServerError> {
        // Read incoming data from TCP stream with proper error handling
        let mut buffer = [0; 1024];
        
        let bytes_read = match stream.read(&mut buffer) {
            Ok(0) => {
                self.logger.log_warning(&format!("Client {} closed connection immediately", client_addr));
                return Ok(());
            }
            Ok(bytes) => {
                self.logger.log_info(&format!("Received {} bytes from {}", bytes, client_addr));
                bytes
            }
            Err(e) => {
                match e.kind() {
                    ErrorKind::TimedOut => {
                        self.logger.log_warning(&format!("Read timeout for client {}", client_addr));
                        let response = HttpResponse::new(408, "Request Timeout")
                            .with_content_type("text/plain")
                            .with_body("Request timed out");
                        let _ = stream.write(response.format().as_bytes());
                        return Err(ServerError::TimeoutError);
                    }
                    ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => {
                        self.logger.log_warning(&format!("Connection reset by client {}", client_addr));
                        return Ok(());
                    }
                    _ => {
                        self.logger.log_error(&format!("Read error from {}: {}", client_addr, e));
                        return Err(ServerError::IoError(e));
                    }
                }
            }
        };

        let request_data = String::from_utf8_lossy(&buffer[..bytes_read]);
        
        // Handle malformed HTTP requests gracefully
        let response = match HttpRequest::parse(&request_data) {
            Ok(request) => {
                self.logger.log_request(&request.method, &request.path, 200, client_addr);
                
                // Use router for request handling
                let response = self.router.route(&request);
                self.logger.log_request(&request.method, &request.path, response.status_code, client_addr);
                response
            }
            Err(parse_error) => {
                // Log errors appropriately
                self.logger.log_warning(&format!("Malformed request from {}: {}", client_addr, parse_error));
                self.logger.log_request("INVALID", "N/A", 400, client_addr);
                
                HttpResponse::new(400, "Bad Request")
                    .with_content_type("text/html")
                    .with_body("<h1>400 - Bad Request</h1><p>The request could not be parsed.</p>")
            }
        };

        // Send response with error handling
        match stream.write(response.format().as_bytes()) {
            Ok(_) => {
                if let Err(e) = stream.flush() {
                    self.logger.log_warning(&format!("Failed to flush response to {}: {}", client_addr, e));
                }
            }
            Err(e) => {
                self.logger.log_error(&format!("Failed to send response to {}: {}", client_addr, e));
                return Err(ServerError::IoError(e));
            }
        }

        Ok(())
    }

    // Legacy method for backward compatibility
    fn handle_connection(&self, stream: TcpStream) {
        // Fallback to safe method
        let client_addr = stream.peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
            
        if let Err(e) = self.handle_connection_safe(stream, &client_addr) {
            self.logger.log_error(&format!("Connection handling failed: {:?}", e));
        }
    }

    // Route handlers
    fn handle_home(request: &HttpRequest) -> HttpResponse {
        let query_params = Router::parse_query_params(&request.path);
        let mut body = String::from("<h1>Welcome to Rust HTTP Server!</h1>");
        body.push_str("<p>Available routes:</p>");
        body.push_str("<ul>");
        body.push_str("<li><a href='/hello'>GET /hello</a></li>");
        body.push_str("<li><a href='/api/status'>GET /api/status</a></li>");
        body.push_str("<li>POST /api/echo</li>");
        body.push_str("</ul>");
        
        if !query_params.is_empty() {
            body.push_str("<h3>Query Parameters:</h3><ul>");
            for (key, value) in query_params {
                body.push_str(&format!("<li>{}: {}</li>", key, value));
            }
            body.push_str("</ul>");
        }
        
        HttpResponse::new(200, "OK")
            .with_content_type("text/html")
            .with_body(&body)
    }

    fn handle_hello(request: &HttpRequest) -> HttpResponse {
        let query_params = Router::parse_query_params(&request.path);
        let default_name = "World".to_string();
        let name = query_params.get("name").unwrap_or(&default_name);
        
        HttpResponse::new(200, "OK")
            .with_content_type("text/plain")
            .with_body(&format!("Hello, {}!", name))
    }

    fn handle_status(_request: &HttpRequest) -> HttpResponse {
        HttpResponse::new(200, "OK")
            .with_content_type("application/json")
            .with_body(r#"{"status":"ok","server":"rust-http-server","version":"1.0.0"}"#)
    }

    fn handle_echo(request: &HttpRequest) -> HttpResponse {
        HttpResponse::new(200, "OK")
            .with_content_type("application/json")
            .with_body(&format!(r#"{{"method":"{}","path":"{}","body":"{}"}}"#, 
                request.method, request.path, request.body))
    }

    // Legacy handlers for backward compatibility (now unused)
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
