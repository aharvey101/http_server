mod server;

use server::HttpServer;

fn main() {
    let server = HttpServer::new("127.0.0.1:8080").unwrap();
    
    if let Err(e) = server.start() {
        eprintln!("Server error: {}", e);
    }
}
