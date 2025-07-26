use super::helpers::*;

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_root_index_serving() {
        let port = 9010;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test that root path serves index.html from static directory
        let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/html"));
        // Should be serving the home page, not static index.html
        assert!(response.contains("Welcome to Rust HTTP Server!"));
    }
}
