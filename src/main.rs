mod server;
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
    
    if let Err(e) = server.start() {
        eprintln!("Server error: {:?}", e);
    }
}
