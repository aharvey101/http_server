use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;
use std::time::Duration;
use std::io::ErrorKind;
use std::sync::Arc;
use super::{
    ServerError, Logger, HttpRequest, HttpResponse, Router, ThreadPool, 
    ConnectionPool, BufferedStream, ServerConfig
};

pub struct HttpServer {
    listener: TcpListener,
    router: Router,
    logger: Logger,
    thread_pool: ThreadPool,
    #[allow(dead_code)] // TODO: implement connection pooling
    connection_pool: ConnectionPool,
    config: ServerConfig,
}

impl HttpServer {
    #[allow(dead_code)] // Public API method
    pub fn new(address: &str) -> Result<Self, ServerError> {
        let config = ServerConfig::default();
        let listener = TcpListener::bind(address)?;
        Self::from_config_and_listener(config, listener)
    }

    pub fn from_config(config: ServerConfig) -> Result<Self, ServerError> {
        let address = config.get_bind_address();
        let listener = TcpListener::bind(&address)?;
        Self::from_config_and_listener(config, listener)
    }

    fn from_config_and_listener(config: ServerConfig, listener: TcpListener) -> Result<Self, ServerError> {
        let mut router = Router::new();
        let logger = Logger::new();
        
        // Initialize thread pool with config values
        let thread_pool = ThreadPool::new(
            config.threading.worker_threads, 
            config.threading.max_concurrent_connections
        );
        
        // Initialize connection pool with config values
        let connection_pool = ConnectionPool::new(
            config.connection.max_idle_connections, 
            config.connection.idle_timeout_seconds
        );
        
        // Configure static files
        if config.static_files.enabled {
            router.set_static_dir(&config.static_files.directory);
        }
        
        // Configure authentication
        if config.authentication.enabled {
            for (username, password) in &config.authentication.users {
                router.add_auth_user(username, password);
            }
            for path in &config.authentication.protected_paths {
                router.add_protected_path(path);
            }
        }
        
        // Add some default routes
        router.add_route("GET", "/", Self::handle_home);
        router.add_route("GET", "/hello", Self::handle_hello);
        router.add_route("GET", "/api/status", Self::handle_status);
        router.add_route("GET", "/api/stats", Self::handle_stats);
        router.add_route("POST", "/api/echo", Self::handle_echo);
        router.add_route("GET", "/admin", Self::handle_admin);
        router.add_route("GET", "/chunked", Self::handle_chunked_demo);
        
        Ok(HttpServer { listener, router, logger, thread_pool, connection_pool, config })
    }

    #[allow(dead_code)] // Public API method
    pub fn add_route(&mut self, method: &str, path: &str, handler: fn(&HttpRequest) -> HttpResponse) {
        self.router.add_route(method, path, handler);
    }

    #[allow(dead_code)] // Public API method
    pub fn set_static_dir(&mut self, dir: &str) {
        self.router.set_static_dir(dir);
    }

    #[allow(dead_code)] // Public API method
    pub fn add_auth_user(&mut self, username: &str, password: &str) {
        self.router.add_auth_user(username, password);
    }

    #[allow(dead_code)] // Public API method  
    pub fn add_auth_user_with_password(&mut self, username: &str, plain_password: &str) {
        self.router.add_auth_user_with_password(username, plain_password);
    }

    #[allow(dead_code)] // Public API method
    pub fn add_protected_path(&mut self, path: &str) {
        self.router.add_protected_path(path);
    }

    #[allow(dead_code)] // Public API method
    pub fn get_config(&self) -> &ServerConfig {
        &self.config
    }

    pub fn start(&self) -> Result<(), ServerError> {
        let addr = self.listener.local_addr()?;
        self.logger.log_info(&format!("HTTP Server starting on http://{}", addr));
        self.logger.log_info(&format!("Thread pool initialized with {} workers", self.config.threading.worker_threads));
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
                    
                    // Add timeout handling for connections using config values
                    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(self.config.server.read_timeout_seconds))) {
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
