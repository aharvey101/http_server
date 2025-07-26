mod server;
#[cfg(test)]
mod tests;

use server::HttpServer;

fn main() {
    let mut server = match HttpServer::new("127.0.0.1:8080") {
        Ok(server) => server,
        Err(e) => {
            eprintln!("Failed to create server: {:?}", e);
            return;
        }
    };
    
    // Enable static file serving
    server.set_static_dir("static");
    
    // Configure authentication
    server.add_auth_user("admin", "password123");
    server.add_auth_user("user", "secret");
    
    // Protect the admin path
    server.add_protected_path("/admin");
    
    println!("🚀 HTTP Server with Advanced Features:");
    println!("   - HTTP/1.1 with keep-alive connections");
    println!("   - Chunked transfer encoding support");
    println!("   - Basic HTTP authentication");
    println!("   - Static file serving with directory listings");
    println!("");
    println!("📋 Available endpoints:");
    println!("   GET  /               - Home page");
    println!("   GET  /hello?name=X   - Greeting with query params");
    println!("   GET  /api/status     - JSON status endpoint");
    println!("   POST /api/echo       - Echo request data");
    println!("   GET  /admin          - Protected admin panel (user: admin, pass: password123)");
    println!("   GET  /chunked        - Chunked encoding demo");
    println!("   GET  /static/        - Static file directory");
    println!("");
    println!("🔧 Test commands:");
    println!("   curl http://127.0.0.1:8080/");
    println!("   curl -u admin:password123 http://127.0.0.1:8080/admin");
    println!("   curl http://127.0.0.1:8080/chunked");
    println!("");
    
    if let Err(e) = server.start() {
        eprintln!("Server error: {:?}", e);
    }
}
