use std::net::TcpStream;
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;
use api::HttpServer;

/// Start a test server on the specified port
pub fn start_test_server(port: u16) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut server = HttpServer::new(&format!("127.0.0.1:{}", port)).unwrap();
        server.set_static_dir("static");
        // Add authentication for testing
        server.add_auth_user_with_password("testuser", "testpass");
        server.add_protected_path("/admin");
        server.start().unwrap();
    })
}

/// Send an HTTP request to the test server and return the response
pub fn send_http_request(port: u16, request: &str) -> String {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    
    // Set a read timeout to prevent hanging
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    
    // Modify request to include Connection: close header if not already present
    let request_with_close = if !request.contains("Connection:") {
        request.replace("\r\n\r\n", "\r\nConnection: close\r\n\r\n")
    } else {
        request.to_string()
    };
    
    stream.write_all(request_with_close.as_bytes()).unwrap();
    
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response); // Ignore errors from connection close
    response
}

/// Wait for the server to start listening on the specified port
pub fn wait_for_server(port: u16) {
    // Wait for server to start
    for _ in 0..50 {
        if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("Server failed to start on port {}", port);
}
