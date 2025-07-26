mod server;
mod tests;

use server::HttpServer;

fn main() {
    let mut server = HttpServer::new("127.0.0.1:8080").unwrap();
    
    // Enable static file serving
    server.set_static_dir("static");
    
    if let Err(e) = server.start() {
        eprintln!("Server error: {}", e);
    }
}
