use std::net::TcpStream;
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;
use crate::server::HttpServer;

#[cfg(test)]
mod tests {
    use super::*;

    fn start_test_server(port: u16) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let server = HttpServer::new(&format!("127.0.0.1:{}", port)).unwrap();
            server.start().unwrap();
        })
    }

    fn send_http_request(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    fn wait_for_server(port: u16) {
        // Wait for server to start
        for _ in 0..50 {
            if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("Server failed to start on port {}", port);
    }

    #[test]
    fn test_get_request() {
        let port = 8081;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/plain"));
        assert!(response.contains("Content-Length:"));
        assert!(response.contains("GET request to path: /hello"));
    }

    #[test]
    fn test_post_request() {
        let port = 8082;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let body = "test data";
        let request = format!(
            "POST /api/data HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response = send_http_request(port, &request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/plain"));
        assert!(response.contains("POST request to path: /api/data"));
        assert!(response.contains("with body: test data"));
    }

    #[test]
    fn test_put_request() {
        let port = 8083;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let body = "updated data";
        let request = format!(
            "PUT /api/update HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response = send_http_request(port, &request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("PUT request to path: /api/update"));
        assert!(response.contains("with body: updated data"));
    }

    #[test]
    fn test_delete_request() {
        let port = 8084;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "DELETE /api/item/123 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("DELETE request to path: /api/item/123"));
    }

    #[test]
    fn test_unsupported_method() {
        let port = 8085;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "PATCH /test HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 405 Method Not Allowed"));
        assert!(response.contains("Method not supported"));
    }

    #[test]
    fn test_malformed_request() {
        let port = 8086;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "INVALID REQUEST\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("Malformed request"));
    }

    #[test]
    fn test_headers_parsing() {
        let port = 8087;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /test HTTP/1.1\r\n\
                      Host: localhost\r\n\
                      User-Agent: test-client\r\n\
                      Accept: text/plain\r\n\
                      \r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("GET request to path: /test"));
    }

    #[test]
    fn test_empty_path() {
        let port = 8088;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("GET request to path: /"));
    }

    #[test]
    fn test_path_with_query_parameters() {
        let port = 8089;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /search?q=rust&type=tutorial HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("GET request to path: /search?q=rust&type=tutorial"));
    }

    #[test]
    fn test_content_length_header() {
        let port = 8090;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /test HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        // Extract content length from response
        let content_length_line = response
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .unwrap();
        
        let content_length: usize = content_length_line
            .split(':')
            .nth(1)
            .unwrap()
            .trim()
            .parse()
            .unwrap();

        // Get the body (everything after the empty line)
        let body_start = response.find("\r\n\r\n").unwrap() + 4;
        let body = &response[body_start..];
        
        assert_eq!(content_length, body.len());
    }
}
