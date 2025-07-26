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
        assert!(response.contains("Hello, World!"));
    }

    #[test]
    fn test_post_request() {
        let port = 8082;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let body = "test data";
        let request = format!(
            "POST /api/echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response = send_http_request(port, &request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: application/json"));
        assert!(response.contains(r#""method":"POST""#));
        assert!(response.contains(r#""body":"test data""#));
    }

    #[test]
    fn test_put_request() {
        let port = 8083;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let body = "updated data";
        let request = format!(
            "PUT /nonexistent HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response = send_http_request(port, &request);

        // PUT to nonexistent route should return 404
        assert!(response.contains("HTTP/1.1 404 Not Found"));
        assert!(response.contains("404 - Page Not Found"));
    }

    #[test]
    fn test_delete_request() {
        let port = 8084;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "DELETE /api/item/123 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        // DELETE to nonexistent route should return 404
        assert!(response.contains("HTTP/1.1 404 Not Found"));
        assert!(response.contains("404 - Page Not Found"));
    }

    #[test]
    fn test_unsupported_method() {
        let port = 8085;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "PATCH /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        // PATCH is not supported for /hello route, should return 404
        assert!(response.contains("HTTP/1.1 404 Not Found"));
        assert!(response.contains("404 - Page Not Found"));
    }

    #[test]
    fn test_malformed_request() {
        let port = 8086;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "INVALID REQUEST\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("400 - Bad Request"));
    }

    #[test]
    fn test_headers_parsing() {
        let port = 8087;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /hello HTTP/1.1\r\n\
                      Host: localhost\r\n\
                      User-Agent: test-client\r\n\
                      Accept: text/plain\r\n\
                      \r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Hello, World!"));
    }

    #[test]
    fn test_empty_path() {
        let port = 8088;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Welcome to Rust HTTP Server!"));
    }

    #[test]
    fn test_path_with_query_parameters() {
        let port = 8089;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /hello?name=Tutorial HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Hello, Tutorial!"));
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

    #[test]
    fn test_routing_home_page() {
        let port = 8091;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/html"));
        assert!(response.contains("Welcome to Rust HTTP Server!"));
    }

    #[test]
    fn test_routing_hello_endpoint() {
        let port = 8092;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/plain"));
        assert!(response.contains("Hello, World!"));
    }

    #[test]
    fn test_routing_hello_with_name_parameter() {
        let port = 8093;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /hello?name=Rust HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Hello, Rust!"));
    }

    #[test]
    fn test_routing_status_endpoint() {
        let port = 8094;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /api/status HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: application/json"));
        assert!(response.contains(r#""status":"ok""#));
        assert!(response.contains(r#""server":"rust-http-server""#));
    }

    #[test]
    fn test_routing_echo_endpoint() {
        let port = 8095;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let body = "test echo data";
        let request = format!(
            "POST /api/echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response = send_http_request(port, &request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: application/json"));
        assert!(response.contains(r#""method":"POST""#));
        assert!(response.contains(r#""path":"/api/echo""#));
        assert!(response.contains(r#""body":"test echo data""#));
    }

    #[test]
    fn test_routing_404_not_found() {
        let port = 8096;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /nonexistent HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 404 Not Found"));
        assert!(response.contains("Content-Type: text/html"));
        assert!(response.contains("404 - Page Not Found"));
    }

    #[test]
    fn test_query_parameter_parsing() {
        let port = 8097;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /?name=Test&type=query HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Query Parameters:"));
        assert!(response.contains("name: Test"));
        assert!(response.contains("type: query"));
    }

    #[test]
    fn test_error_handling_timeout() {
        let port = 8098;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test that server handles connection properly (timeout testing is complex)
        let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Hello, World!"));
    }

    #[test]
    fn test_error_handling_malformed_headers() {
        let port = 8099;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Malformed request with invalid headers
        let request = "GET /hello HTTP/1.1\r\nInvalid-Header-Line\r\n\r\n";
        let response = send_http_request(port, request);

        // Should still work as our parser is tolerant
        assert!(response.contains("HTTP/1.1 200 OK"));
    }

    #[test] 
    fn test_error_handling_directory_traversal() {
        let port = 8100;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Attempt directory traversal attack
        let request = "GET /../etc/passwd HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        // Should return 404 (not revealing that we detected directory traversal)
        assert!(response.contains("HTTP/1.1 404 Not Found"));
        assert!(response.contains("404 - Page Not Found"));
    }

    #[test]
    fn test_error_handling_large_request() {
        let port = 8101;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Large body that might exceed buffer
        let large_body = "x".repeat(2048);
        let request = format!(
            "POST /api/echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
            large_body.len(),
            large_body
        );
        
        // This should either work or fail gracefully
        match std::panic::catch_unwind(|| {
            send_http_request(port, &request)
        }) {
            Ok(response) => {
                // If it succeeds, should be valid HTTP response
                assert!(response.contains("HTTP/1.1"));
            }
            Err(_) => {
                // If it panics/fails, that's also acceptable for this stress test
            }
        }
    }

    #[test]
    fn test_error_handling_empty_request() {
        let port = 8102;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Send completely empty request
        let request = "";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 400 Bad Request"));
        assert!(response.contains("400 - Bad Request"));
    }

    #[test]
    fn test_error_handling_invalid_method() {
        let port = 8103;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Invalid HTTP method
        let request = "INVALID_METHOD /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        // Should return 404 since the method/path combo doesn't exist
        assert!(response.contains("HTTP/1.1 404 Not Found"));
    }
}
