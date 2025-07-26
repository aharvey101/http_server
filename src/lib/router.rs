use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use super::{
    HttpRequest, HttpResponse, Route, base64_decode, verify_password, 
    hash_password, generate_salt, TokenManager, parse_login_request,
    create_login_response, create_error_response
};

pub struct Router {
    routes: Vec<Route>,
    static_dir: Option<String>,
    auth_users: Arc<Mutex<HashMap<String, String>>>, // username -> password_hash
    protected_paths: Vec<String>,
    token_manager: Arc<TokenManager>,
}

impl Clone for Router {
    fn clone(&self) -> Self {
        Router {
            routes: self.routes.clone(),
            static_dir: self.static_dir.clone(),
            auth_users: Arc::clone(&self.auth_users),
            protected_paths: self.protected_paths.clone(),
            token_manager: Arc::clone(&self.token_manager),
        }
    }
}

impl Router {
    pub fn new() -> Self {
        Router {
            routes: Vec::new(),
            static_dir: None,
            auth_users: Arc::new(Mutex::new(HashMap::new())),
            protected_paths: Vec::new(),
            token_manager: Arc::new(TokenManager::new()),
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

    pub fn add_auth_user(&self, username: &str, password: &str) {
        if let Ok(mut auth_users) = self.auth_users.lock() {
            auth_users.insert(username.to_string(), password.to_string());
        }
    }

    // Add a user with automatic password hashing
    pub fn add_auth_user_with_password(&self, username: &str, plain_password: &str) {
        let salt = generate_salt();
        let hashed_password = hash_password(plain_password, &salt);
        if let Ok(mut auth_users) = self.auth_users.lock() {
            auth_users.insert(username.to_string(), hashed_password);
        }
    }

    pub fn add_protected_path(&mut self, path: &str) {
        self.protected_paths.push(path.to_string());
    }

    // Authentication helper - supports both Basic Auth and Bearer Token
    fn authenticate(&self, request: &HttpRequest) -> bool {
        if let Some(auth_header) = request.headers.get("authorization") {
            if auth_header.starts_with("Bearer ") {
                // Token-based authentication
                let token = &auth_header[7..]; // Skip "Bearer "
                return self.token_manager.validate_token(token).is_some();
            } else if auth_header.starts_with("Basic ") {
                // Basic authentication
                let encoded = &auth_header[6..]; // Skip "Basic "
                
                // Decode base64 credentials (simplified implementation)
                if let Ok(decoded_bytes) = base64_decode(encoded) {
                    if let Ok(decoded) = String::from_utf8(decoded_bytes) {
                        if let Some(colon_pos) = decoded.find(':') {
                            let username = &decoded[..colon_pos];
                            let password = &decoded[colon_pos + 1..];
                            
                            if let Ok(auth_users) = self.auth_users.lock() {
                                return auth_users.get(username)
                                    .map(|stored_hash| verify_password(password, stored_hash))
                                    .unwrap_or(false);
                            }
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

        // Handle authentication endpoints
        match path_without_query {
            "/api/register" => return self.handle_register(request),
            "/api/login" => return self.handle_login(request),
            "/api/logout" => return self.handle_logout(request),
            _ => {}
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

    /// Handle user registration endpoint
    pub fn handle_register(&self, request: &HttpRequest) -> HttpResponse {
        if request.method != "POST" {
            return HttpResponse::new(405, "Method Not Allowed")
                .with_content_type("application/json")
                .with_body(&create_error_response("Only POST method allowed"));
        }

        // Parse JSON body
        if let Some((username, password)) = parse_login_request(&request.body) {
            // Check if user already exists
            if let Ok(auth_users) = self.auth_users.lock() {
                if auth_users.contains_key(&username) {
                    return HttpResponse::new(409, "Conflict")
                        .with_content_type("application/json")
                        .with_body(&create_error_response("Username already exists"));
                }
            }

            // Hash the password and store the user
            let salt = generate_salt();
            let password_hash = hash_password(&password, &salt);
            if let Ok(mut auth_users) = self.auth_users.lock() {
                auth_users.insert(username.clone(), password_hash);
            }

            // Generate a token for the new user
            let token = self.token_manager.generate_token(&username);
            
            HttpResponse::new(201, "Created")
                .with_content_type("application/json")
                .with_body(&create_login_response(&token))
        } else {
            HttpResponse::new(400, "Bad Request")
                .with_content_type("application/json")
                .with_body(&create_error_response("Invalid JSON format. Expected {\"username\": \"...\", \"password\": \"...\"}"))
        }
    }

    /// Handle user login endpoint
    pub fn handle_login(&self, request: &HttpRequest) -> HttpResponse {
        if request.method != "POST" {
            return HttpResponse::new(405, "Method Not Allowed")
                .with_content_type("application/json")
                .with_body(&create_error_response("Only POST method allowed"));
        }

        // Parse JSON body
        if let Some((username, password)) = parse_login_request(&request.body) {
            // Verify credentials
            if let Ok(auth_users) = self.auth_users.lock() {
                if let Some(stored_hash) = auth_users.get(&username) {
                    if verify_password(&password, stored_hash) {
                        // Generate a token for the user
                        let token = self.token_manager.generate_token(&username);
                        
                        return HttpResponse::new(200, "OK")
                            .with_content_type("application/json")
                            .with_body(&create_login_response(&token));
                    }
                }
            }
            
            HttpResponse::new(401, "Unauthorized")
                .with_content_type("application/json")
                .with_body(&create_error_response("Invalid username or password"))
        } else {
            HttpResponse::new(400, "Bad Request")
                .with_content_type("application/json")
                .with_body(&create_error_response("Invalid JSON format. Expected {\"username\": \"...\", \"password\": \"...\"}"))
        }
    }

    /// Handle token logout endpoint
    pub fn handle_logout(&self, request: &HttpRequest) -> HttpResponse {
        if request.method != "POST" {
            return HttpResponse::new(405, "Method Not Allowed")
                .with_content_type("application/json")
                .with_body(&create_error_response("Only POST method allowed"));
        }

        // Extract token from Authorization header
        if let Some(auth_header) = request.headers.get("authorization") {
            if auth_header.starts_with("Bearer ") {
                let token = &auth_header[7..]; // Skip "Bearer "
                
                if self.token_manager.revoke_token(token) {
                    return HttpResponse::new(200, "OK")
                        .with_content_type("application/json")
                        .with_body(r#"{"success": true, "message": "Logged out successfully"}"#);
                }
            }
        }
        
        HttpResponse::new(400, "Bad Request")
            .with_content_type("application/json")
            .with_body(&create_error_response("Invalid or missing token"))
    }
}
