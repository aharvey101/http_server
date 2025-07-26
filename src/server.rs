use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::io::{self, ErrorKind};
use std::thread;
use std::sync::{Arc, Mutex, mpsc};
use std::sync::atomic::{AtomicUsize, Ordering};

// Simple base64 decoder for authentication (simplified implementation)
fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();
    let input = input.trim();
    
    if input.len() % 4 != 0 {
        return Err("Invalid base64 length");
    }
    
    for chunk in input.as_bytes().chunks(4) {
        let mut values = [0u8; 4];
        
        for (i, &byte) in chunk.iter().enumerate() {
            if byte == b'=' {
                values[i] = 0;
            } else if let Some(pos) = CHARS.iter().position(|&c| c == byte) {
                values[i] = pos as u8;
            } else {
                return Err("Invalid base64 character");
            }
        }
        
        let combined = ((values[0] as u32) << 18) | 
                      ((values[1] as u32) << 12) | 
                      ((values[2] as u32) << 6) | 
                      (values[3] as u32);
        
        result.push((combined >> 16) as u8);
        if chunk[2] != b'=' {
            result.push((combined >> 8) as u8);
        }
        if chunk[3] != b'=' {
            result.push(combined as u8);
        }
    }
    
    Ok(result)
}

// Custom error types for better error handling
#[derive(Debug)]
pub enum ServerError {
    IoError(io::Error),
    TimeoutError,
    ConnectionError(String),
}

impl From<io::Error> for ServerError {
    fn from(error: io::Error) -> Self {
        ServerError::IoError(error)
    }
}

// Logger for comprehensive logging
pub struct Logger {
}

impl Logger {
    pub fn new() -> Self {
        Logger {
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

#[derive(Debug, Clone)]
pub struct Route {
    pub method: String,
    pub path: String,
    pub handler: fn(&HttpRequest) -> HttpResponse,
}

#[derive(Clone)]
pub struct Router {
    routes: Vec<Route>,
    static_dir: Option<String>,
    auth_users: HashMap<String, String>, // username -> password
    protected_paths: Vec<String>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            routes: Vec::new(),
            static_dir: None,
            auth_users: HashMap::new(),
            protected_paths: Vec::new(),
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

    pub fn add_auth_user(&mut self, username: &str, password: &str) {
        self.auth_users.insert(username.to_string(), password.to_string());
    }

    pub fn add_protected_path(&mut self, path: &str) {
        self.protected_paths.push(path.to_string());
    }

    // Basic HTTP Authentication helper
    fn authenticate(&self, request: &HttpRequest) -> bool {
        if let Some(auth_header) = request.headers.get("authorization") {
            if auth_header.starts_with("Basic ") {
                let encoded = &auth_header[6..]; // Skip "Basic "
                
                // Decode base64 credentials (simplified implementation)
                if let Ok(decoded_bytes) = base64_decode(encoded) {
                    if let Ok(decoded) = String::from_utf8(decoded_bytes) {
                        if let Some(colon_pos) = decoded.find(':') {
                            let username = &decoded[..colon_pos];
                            let password = &decoded[colon_pos + 1..];
                            
                            return self.auth_users.get(username)
                                .map(|stored_password| stored_password == password)
                                .unwrap_or(false);
                        }
                    }
                }
            }
        }
        false
    }

    fn is_protected_path(&self, path: &str) -> bool {
        self.protected_paths.iter().any(|protected| path.starts_with(protected))
    }

    // Create route matching logic
    pub fn route(&self, request: &HttpRequest) -> HttpResponse {
        // Extract path without query parameters for routing
        let path_without_query = if let Some(query_start) = request.path.find('?') {
            &request.path[..query_start]
        } else {
            &request.path
        };

        // Check if path requires authentication
        if self.is_protected_path(path_without_query) {
            if !self.authenticate(request) {
                return HttpResponse::new(401, "Unauthorized")
                    .with_content_type("text/html")
                    .with_header("WWW-Authenticate", "Basic realm=\"Protected Area\"")
                    .with_body("<h1>401 - Unauthorized</h1><p>Authentication required to access this resource.</p>");
            }
        }

        // Handle static file serving first for any path starting with static directory
        if request.method == "GET" && self.static_dir.is_some() {
            if let Some(static_dir) = &self.static_dir {
                // Check if path starts with static directory or is accessing static content
                if path_without_query.starts_with(&format!("/{}/", static_dir)) || path_without_query == format!("/{}", static_dir) {
                    if let Some(response) = self.serve_static_file(path_without_query) {
                        return response;
                    }
                }
            }
        }

        // Handle different URL paths - exact match
        for route in &self.routes {
            if route.method == request.method && route.path == path_without_query {
                return (route.handler)(request);
            }
        }

        // Handle static file serving for root and other paths
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

    // Handle static file serving with enhanced error handling and directory listing
    fn serve_static_file(&self, path: &str) -> Option<HttpResponse> {
        if let Some(static_dir) = &self.static_dir {
            let file_path = if path == "/" {
                format!("{}/index.html", static_dir)
            } else if path == format!("/{}", static_dir) || path == format!("/{}/", static_dir) {
                // Handle requests to the static directory itself
                static_dir.to_string()
            } else if path.starts_with(&format!("/{}/", static_dir)) {
                // Handle requests to files/directories within static directory
                format!("{}{}", static_dir, &path[static_dir.len() + 1..])
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

            let path_obj = Path::new(&file_path);
            
            if path_obj.exists() {
                // If it's a directory, serve directory listing
                if path_obj.is_dir() {
                    return self.serve_directory_listing(&file_path, path);
                }
                
                // If it's a file, serve the file content
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

    // Add directory listing functionality
    fn serve_directory_listing(&self, dir_path: &str, request_path: &str) -> Option<HttpResponse> {
        match fs::read_dir(dir_path) {
            Ok(entries) => {
                let mut html = String::from("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
                html.push_str("<meta charset=\"UTF-8\">\n");
                html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
                html.push_str(&format!("<title>Directory Listing: {}</title>\n", request_path));
                html.push_str("<style>\n");
                html.push_str("body { font-family: Arial, sans-serif; margin: 40px; }\n");
                html.push_str("h1 { color: #d73502; }\n");
                html.push_str("ul { list-style-type: none; padding: 0; }\n");
                html.push_str("li { margin: 5px 0; }\n");
                html.push_str("a { text-decoration: none; color: #0066cc; }\n");
                html.push_str("a:hover { text-decoration: underline; }\n");
                html.push_str(".directory { font-weight: bold; }\n");
                html.push_str(".file { color: #333; }\n");
                html.push_str("</style>\n");
                html.push_str("</head>\n<body>\n");
                html.push_str(&format!("<h1>📁 Directory Listing: {}</h1>\n", request_path));
                
                // Add navigation back to parent directory if not at root
                if request_path != "/" && request_path != "" {
                    let parent_path = if request_path.ends_with('/') {
                        &request_path[..request_path.len()-1]
                    } else {
                        request_path
                    };
                    
                    if let Some(last_slash) = parent_path.rfind('/') {
                        let parent = if last_slash == 0 { "/" } else { &parent_path[..last_slash] };
                        html.push_str(&format!("<p><a href=\"{}\" class=\"directory\">⬆️ Parent Directory</a></p>\n", parent));
                    }
                }
                
                html.push_str("<ul>\n");
                
                // Collect and sort directory entries
                let mut entries_vec: Vec<_> = entries.filter_map(|entry| entry.ok()).collect();
                entries_vec.sort_by(|a, b| {
                    // Sort directories first, then files, both alphabetically
                    let a_is_dir = a.path().is_dir();
                    let b_is_dir = b.path().is_dir();
                    
                    match (a_is_dir, b_is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.file_name().cmp(&b.file_name()),
                    }
                });
                
                for entry in entries_vec {
                    if let Some(name) = entry.file_name().to_str() {
                        let is_dir = entry.path().is_dir();
                        let link_path = if request_path.ends_with('/') {
                            format!("{}{}", request_path, name)
                        } else {
                            format!("{}/{}", request_path, name)
                        };
                        
                        let icon = if is_dir { "📁" } else { "📄" };
                        let class = if is_dir { "directory" } else { "file" };
                        let suffix = if is_dir { "/" } else { "" };
                        
                        html.push_str(&format!(
                            "<li><a href=\"{}{}\" class=\"{}\">{} {}{}</a></li>\n",
                            link_path, suffix, class, icon, name, suffix
                        ));
                    }
                }
                
                html.push_str("</ul>\n");
                html.push_str("<hr>\n");
                html.push_str("<p><em>Generated by Rust HTTP Server</em></p>\n");
                html.push_str("</body>\n</html>");
                
                Some(
                    HttpResponse::new(200, "OK")
                        .with_content_type("text/html")
                        .with_body(&html)
                )
            }
            Err(e) => {
                eprintln!("Directory read error for {}: {}", dir_path, e);
                Some(
                    HttpResponse::new(500, "Internal Server Error")
                        .with_content_type("text/html")
                        .with_body("<h1>500 - Internal Server Error</h1><p>Unable to read directory contents.</p>")
                )
            }
        }
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

// =======================
// STEP 9: PERFORMANCE OPTIMIZATION - THREAD POOL
// =======================

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob(job) => {
                        println!("Worker {} got a job; executing.", id);
                        job();
                    }
                    Message::Terminate => {
                        println!("Worker {} was told to terminate.", id);
                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
    active_connections: Arc<AtomicUsize>,
    max_connections: usize,
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    /// max_connections is the maximum number of concurrent connections allowed.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize, max_connections: usize) -> ThreadPool {
        assert!(size > 0);
        assert!(max_connections > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);
        let active_connections = Arc::new(AtomicUsize::new(0));

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { 
            workers, 
            sender,
            active_connections,
            max_connections,
        }
    }

    pub fn execute<F>(&self, f: F) -> Result<(), &'static str>
    where
        F: FnOnce() + Send + 'static,
    {
        // Check if we've reached the maximum number of connections
        let current_connections = self.active_connections.load(Ordering::SeqCst);
        if current_connections >= self.max_connections {
            return Err("Maximum connections reached");
        }

        // Increment connection counter
        self.active_connections.fetch_add(1, Ordering::SeqCst);

        let active_connections = Arc::clone(&self.active_connections);
        let job = Box::new(move || {
            f();
            // Decrement connection counter when job is done
            active_connections.fetch_sub(1, Ordering::SeqCst);
        });

        self.sender.send(Message::NewJob(job)).unwrap();
        Ok(())
    }

    pub fn get_active_connections(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    pub fn get_max_connections(&self) -> usize {
        self.max_connections
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

// =======================
// STEP 9: PERFORMANCE OPTIMIZATION - CONNECTION POOL (PLACEHOLDER)
// =======================

// Simplified connection pool for future implementation
pub struct ConnectionPool {
}

impl ConnectionPool {
    pub fn new(_max_idle_connections: usize, _idle_timeout_secs: u64) -> Self {
        ConnectionPool {
        }
    }
}

// =======================
// STEP 9: PERFORMANCE OPTIMIZATION - BUFFERED I/O
// =======================

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

pub struct HttpServer {
    listener: TcpListener,
    router: Router,
    logger: Logger,
    thread_pool: ThreadPool,
    connection_pool: ConnectionPool,
}

impl HttpServer {
    pub fn new(address: &str) -> Result<Self, ServerError> {
        let listener = TcpListener::bind(address)?;
        let mut router = Router::new();
        let logger = Logger::new();
        
        // Initialize thread pool with 4 worker threads and max 100 concurrent connections
        let thread_pool = ThreadPool::new(4, 100);
        
        // Initialize connection pool with max 20 idle connections and 30 second timeout
        let connection_pool = ConnectionPool::new(20, 30);
        
        // Add some default routes
        router.add_route("GET", "/", Self::handle_home);
        router.add_route("GET", "/hello", Self::handle_hello);
        router.add_route("GET", "/api/status", Self::handle_status);
        router.add_route("GET", "/api/stats", Self::handle_stats);
        router.add_route("POST", "/api/echo", Self::handle_echo);
        router.add_route("GET", "/admin", Self::handle_admin);
        router.add_route("GET", "/chunked", Self::handle_chunked_demo);
        
        Ok(HttpServer { listener, router, logger, thread_pool, connection_pool })
    }

    pub fn add_route(&mut self, method: &str, path: &str, handler: fn(&HttpRequest) -> HttpResponse) {
        self.router.add_route(method, path, handler);
    }

    pub fn set_static_dir(&mut self, dir: &str) {
        self.router.set_static_dir(dir);
    }

    pub fn add_auth_user(&mut self, username: &str, password: &str) {
        self.router.add_auth_user(username, password);
    }

    pub fn add_protected_path(&mut self, path: &str) {
        self.router.add_protected_path(path);
    }

    pub fn start(&self) -> Result<(), ServerError> {
        let addr = self.listener.local_addr()?;
        self.logger.log_info(&format!("HTTP Server starting on http://{}", addr));
        self.logger.log_info(&format!("Thread pool initialized with {} workers", 4));
        self.logger.log_info(&format!("Maximum concurrent connections: {}", self.thread_pool.get_max_connections()));
        
        // Set read timeout for connections to handle timeout errors
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    // Get client address for logging
                    let client_addr = stream.peer_addr()
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|_| "unknown".to_string());
                    
                    self.logger.log_info(&format!("New connection from {} (Active: {})", 
                        client_addr, self.thread_pool.get_active_connections()));
                    
                    // Add timeout handling for connections
                    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(30))) {
                        self.logger.log_warning(&format!("Failed to set read timeout: {}", e));
                    }
                    
                    // Use thread pool to handle connection concurrently
                    let router = Arc::new(self.router.clone());
                    let logger = Arc::new(Logger::new());
                    let client_addr_clone = client_addr.clone();
                    
                    // Try to clone the stream for the rejection case
                    let stream_clone = match stream.try_clone() {
                        Ok(cloned) => Some(cloned),
                        Err(_) => None,
                    };
                    
                    match self.thread_pool.execute(move || {
                        if let Err(e) = Self::handle_connection_threaded(stream, &client_addr_clone, router, logger) {
                            eprintln!("Connection error for {}: {:?}", client_addr_clone, e);
                        }
                    }) {
                        Ok(()) => {
                            // Connection successfully queued for processing
                        }
                        Err(err) => {
                            self.logger.log_warning(&format!("Connection rejected from {}: {}", client_addr, err));
                            // Send 503 Service Unavailable and close connection if we have a stream clone
                            if let Some(mut reject_stream) = stream_clone {
                                let response = HttpResponse::new(503, "Service Unavailable")
                                    .with_content_type("text/html")
                                    .with_connection("close")
                                    .with_body("<h1>503 - Service Unavailable</h1><p>Server is too busy to handle your request.</p>");
                                let _ = reject_stream.write_all(response.format().as_bytes());
                            }
                        }
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

    // Enhanced connection handling with comprehensive error handling and HTTP keep-alive support
    fn handle_connection_safe(&self, mut stream: TcpStream, client_addr: &str) -> Result<(), ServerError> {
        // Support multiple requests per connection (HTTP keep-alive)
        loop {
            // Read incoming data from TCP stream with proper error handling
            let mut buffer = [0; 1024];
            
            let bytes_read = match stream.read(&mut buffer) {
                Ok(0) => {
                    self.logger.log_info(&format!("Client {} closed connection", client_addr));
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
                        ErrorKind::WouldBlock => {
                            // No data available right now, continue to next iteration
                            continue;
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
            let (response, should_keep_alive) = match HttpRequest::parse(&request_data) {
                Ok(request) => {
                    // Check if client wants to keep connection alive
                    let connection_header = request.headers.get("connection")
                        .map(|s| s.to_lowercase())
                        .unwrap_or_else(|| {
                            // Default behavior based on HTTP version
                            if request.version == "HTTP/1.1" {
                                "keep-alive".to_string()
                            } else {
                                "close".to_string()
                            }
                        });
                    
                    let keep_alive = connection_header.contains("keep-alive");
                    
                    self.logger.log_request(&request.method, &request.path, 200, client_addr);
                    
                    // Use router for request handling
                    let mut response = self.router.route(&request);
                    
                    // Add connection header to response
                    if keep_alive {
                        response = response.with_connection("keep-alive");
                    } else {
                        response = response.with_connection("close");
                    }
                    
                    // Check if client accepts chunked encoding
                    let supports_chunked = request.headers.get("te")
                        .map(|encoding| encoding.contains("chunked"))
                        .unwrap_or(true); // Default to supporting chunked for HTTP/1.1
                    
                    self.logger.log_request(&request.method, &request.path, response.status_code, client_addr);
                    (response, keep_alive && supports_chunked)
                }
                Err(parse_error) => {
                    // Log errors appropriately
                    self.logger.log_warning(&format!("Malformed request from {}: {}", client_addr, parse_error));
                    self.logger.log_request("INVALID", "N/A", 400, client_addr);
                    
                    let response = HttpResponse::new(400, "Bad Request")
                        .with_content_type("text/html")
                        .with_connection("close")
                        .with_body("<h1>400 - Bad Request</h1><p>The request could not be parsed.</p>");
                    (response, false)
                }
            };

            // Send response with error handling
            let formatted_response = if should_keep_alive && response.headers.contains_key("Transfer-Encoding") {
                // Use chunked encoding if explicitly requested
                response.format_chunked()
            } else {
                response.format()
            };

            match stream.write(formatted_response.as_bytes()) {
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

            // Check if we should close the connection
            if !should_keep_alive || response.headers.get("Connection").map(|c| c.to_lowercase().contains("close")).unwrap_or(false) {
                self.logger.log_info(&format!("Closing connection to {}", client_addr));
                break;
            }
        }

        Ok(())
    }

    // New threaded connection handler for use with thread pool
    fn handle_connection_threaded(
        stream: TcpStream, 
        client_addr: &str, 
        router: Arc<Router>, 
        logger: Arc<Logger>
    ) -> Result<(), ServerError> {
        // Use buffered I/O for better performance
        let mut buffered_stream = BufferedStream::new(stream.try_clone().unwrap(), 8192);
        
        // Support multiple requests per connection (HTTP keep-alive)
        loop {
            // Read incoming HTTP request using buffered I/O
            let request_data = match buffered_stream.read_request() {
                Ok(data) => {
                    if data.trim().is_empty() {
                        logger.log_info(&format!("Client {} closed connection", client_addr));
                        return Ok(());
                    }
                    logger.log_info(&format!("Received request from {}", client_addr));
                    data
                }
                Err(e) => {
                    match e.kind() {
                        ErrorKind::TimedOut => {
                            logger.log_warning(&format!("Read timeout for client {}", client_addr));
                            let response = HttpResponse::new(408, "Request Timeout")
                                .with_content_type("text/plain")
                                .with_body("Request timed out");
                            let _ = buffered_stream.write_response(&response.format());
                            let _ = buffered_stream.flush();
                            return Err(ServerError::TimeoutError);
                        }
                        ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => {
                            logger.log_warning(&format!("Connection reset by client {}", client_addr));
                            return Ok(());
                        }
                        ErrorKind::UnexpectedEof => {
                            logger.log_info(&format!("Client {} closed connection", client_addr));
                            return Ok(());
                        }
                        _ => {
                            logger.log_error(&format!("Read error from {}: {}", client_addr, e));
                            return Err(ServerError::IoError(e));
                        }
                    }
                }
            };
            
            // Handle malformed HTTP requests gracefully
            let (response, should_keep_alive) = match HttpRequest::parse(&request_data) {
                Ok(request) => {
                    // Check if client wants to keep connection alive
                    let connection_header = request.headers.get("connection")
                        .map(|s| s.to_lowercase())
                        .unwrap_or_else(|| {
                            // Default behavior based on HTTP version
                            if request.version == "HTTP/1.1" {
                                "keep-alive".to_string()
                            } else {
                                "close".to_string()
                            }
                        });
                    
                    let keep_alive = connection_header.contains("keep-alive");
                    
                    // Use router for request handling
                    let mut response = router.route(&request);
                    
                    // Add connection header to response
                    if keep_alive {
                        response = response.with_connection("keep-alive");
                    } else {
                        response = response.with_connection("close");
                    }
                    
                    // Check if client accepts chunked encoding
                    let supports_chunked = request.headers.get("te")
                        .map(|encoding| encoding.contains("chunked"))
                        .unwrap_or(true); // Default to supporting chunked for HTTP/1.1
                    
                    logger.log_request(&request.method, &request.path, response.status_code, client_addr);
                    (response, keep_alive && supports_chunked)
                }
                Err(parse_error) => {
                    // Log errors appropriately
                    logger.log_warning(&format!("Malformed request from {}: {}", client_addr, parse_error));
                    logger.log_request("INVALID", "N/A", 400, client_addr);
                    
                    let response = HttpResponse::new(400, "Bad Request")
                        .with_content_type("text/html")
                        .with_connection("close")
                        .with_body("<h1>400 - Bad Request</h1><p>The request could not be parsed.</p>");
                    (response, false)
                }
            };

            // Send response with buffered I/O
            let formatted_response = if should_keep_alive && response.headers.contains_key("Transfer-Encoding") {
                // Use chunked encoding if explicitly requested
                response.format_chunked()
            } else {
                response.format()
            };

            match buffered_stream.write_response(&formatted_response) {
                Ok(_) => {
                    if let Err(e) = buffered_stream.flush() {
                        logger.log_warning(&format!("Failed to flush response to {}: {}", client_addr, e));
                    }
                }
                Err(e) => {
                    logger.log_error(&format!("Failed to send response to {}: {}", client_addr, e));
                    return Err(ServerError::IoError(e));
                }
            }

            // Check if we should close the connection
            if !should_keep_alive || response.headers.get("Connection").map(|c| c.to_lowercase().contains("close")).unwrap_or(false) {
                logger.log_info(&format!("Closing connection to {}", client_addr));
                break;
            }
        }

        Ok(())
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

    fn handle_stats(_request: &HttpRequest) -> HttpResponse {
        // For a static method, we can't access instance data like thread_pool
        // In a real implementation, you'd use a shared state (Arc<Mutex<Stats>>)
        let stats = r#"{
            "server": "rust-http-server-optimized",
            "version": "1.0.0",
            "features": {
                "multi_threading": true,
                "connection_pooling": true,
                "buffered_io": true,
                "keep_alive": true,
                "chunked_encoding": true,
                "authentication": true
            },
            "performance": {
                "thread_pool_size": 4,
                "max_connections": 100,
                "buffer_size": "8KB",
                "connection_timeout": "30s"
            }
        }"#;
        
        HttpResponse::new(200, "OK")
            .with_content_type("application/json")
            .with_body(stats)
    }

    fn handle_echo(request: &HttpRequest) -> HttpResponse {
        HttpResponse::new(200, "OK")
            .with_content_type("application/json")
            .with_body(&format!(r#"{{"method":"{}","path":"{}","body":"{}"}}"#, 
                request.method, request.path, request.body))
    }

    fn handle_admin(_request: &HttpRequest) -> HttpResponse {
        HttpResponse::new(200, "OK")
            .with_content_type("text/html")
            .with_body("<h1>🔒 Admin Panel</h1><p>Welcome to the protected admin area!</p><p>You successfully authenticated.</p>")
    }

    fn handle_chunked_demo(_request: &HttpRequest) -> HttpResponse {
        let large_content = "This is a demonstration of chunked transfer encoding. ".repeat(20);
        HttpResponse::new(200, "OK")
            .with_content_type("text/plain")
            .with_chunked_encoding()
            .with_body(&large_content)
    }
}
