use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;

pub struct HttpServer {
    listener: TcpListener,
}

impl HttpServer {
    pub fn new(address: &str) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(address)?;
        Ok(HttpServer { listener })
    }

    pub fn start(&self) -> Result<(), std::io::Error> {
        println!("HTTP Server listening on http://{}", self.listener.local_addr()?);

        // Implement basic connection acceptance loop
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection established!");
                    self.handle_connection(stream);
                }
                Err(e) => {
                    println!("Error accepting connection: {}", e);
                }
            }
        }
        Ok(())
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        // Read incoming data from TCP stream (basic implementation)
        let mut buffer = [0; 1024];
        
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                println!("Received {} bytes", bytes_read);
                let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                println!("Request:\n{}", request);
                
                // Send a simple response for now
                let response = "HTTP/1.1 200 OK\r\n\r\nHello from Rust TCP Server!";
                stream.write(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
            Err(e) => {
                println!("Error reading from connection: {}", e);
            }
        }
    }
}
