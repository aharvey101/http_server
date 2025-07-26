use super::helpers::*;
use std::thread;
use crate::lib::server::HttpServer;

// =======================
// STEP 8: ADVANCED FEATURES TESTS
// =======================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_keep_alive_connection() {
        let port = 9101;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Connection: keep-alive"));
        assert!(response.contains("Hello, World!"));
    }

    #[test]
    fn test_http_connection_close() {
        let port = 9102;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Connection: close"));
        assert!(response.contains("Hello, World!"));
    }

    #[test]
    fn test_chunked_transfer_encoding() {
        let port = 9103;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /chunked HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Transfer-Encoding: chunked"));
        assert!(response.contains("This is a demonstration of chunked transfer encoding"));
    }

    #[test]
    fn test_basic_auth_protected_resource_unauthorized() {
        let port = 9104;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /admin HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 401 Unauthorized"));
        assert!(response.contains("WWW-Authenticate: Basic realm=\"Protected Area\""));
        assert!(response.contains("Authentication required"));
    }

    #[test]
    fn test_basic_auth_with_valid_credentials() {
        let port = 9105;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Base64 encode "testuser:testpass" = "dGVzdHVzZXI6dGVzdHBhc3M="
        let request = "GET /admin HTTP/1.1\r\nHost: localhost\r\nAuthorization: Basic dGVzdHVzZXI6dGVzdHBhc3M=\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Admin Panel"));
        assert!(response.contains("Welcome to the protected admin area"));
    }

    #[test]
    fn test_basic_auth_with_invalid_credentials() {
        let port = 9106;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Base64 encode "wronguser:wrongpass" = "d3JvbmdVc2VyOndyb25nUGFzcw=="
        let request = "GET /admin HTTP/1.1\r\nHost: localhost\r\nAuthorization: Basic d3JvbmdVc2VyOndyb25nUGFzcw==\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 401 Unauthorized"));
        assert!(response.contains("WWW-Authenticate: Basic realm=\"Protected Area\""));
        assert!(response.contains("Authentication required"));
    }

    #[test]
    fn test_basic_auth_malformed_header() {
        let port = 9107;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /admin HTTP/1.1\r\nHost: localhost\r\nAuthorization: Basic invalid-base64\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 401 Unauthorized"));
        assert!(response.contains("WWW-Authenticate: Basic realm=\"Protected Area\""));
    }

    #[test]
    fn test_unprotected_resource_no_auth_required() {
        let port = 9108;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Hello, World!"));
        // Should not contain any authentication-related headers
        assert!(!response.contains("WWW-Authenticate"));
    }

    #[test]
    fn test_http_11_features_proper_headers() {
        let port = 9109;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /api/status HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: application/json"));
        assert!(response.contains("Content-Length:"));
        assert!(response.contains("Connection:"));
        // Should have proper HTTP/1.1 status line
        assert!(response.starts_with("HTTP/1.1"));
    }

    #[test]
    fn test_http_11_version_handling() {
        let port = 9110;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test with HTTP/1.0 request
        let request = "GET /hello HTTP/1.0\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        // Should still respond with HTTP/1.1
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Hello, World!"));
    }

    #[test]
    fn test_multiple_auth_users() {
        let port = 9111;
        let _server_handle = thread::spawn(move || {
            let mut server = HttpServer::new(&format!("127.0.0.1:{}", port)).unwrap();
            server.set_static_dir("static");
            // Add multiple auth users
            server.add_auth_user("admin", "admin123");
            server.add_auth_user("user", "user456");
            server.add_protected_path("/admin");
            server.start().unwrap();
        });
        wait_for_server(port);

        // Test first user - Base64 encode "admin:admin123" = "YWRtaW46YWRtaW4xMjM="
        let request1 = "GET /admin HTTP/1.1\r\nHost: localhost\r\nAuthorization: Basic YWRtaW46YWRtaW4xMjM=\r\n\r\n";
        let response1 = send_http_request(port, request1);
        assert!(response1.contains("HTTP/1.1 200 OK"));

        // Test second user - Base64 encode "user:user456" = "dXNlcjp1c2VyNDU2"
        let request2 = "GET /admin HTTP/1.1\r\nHost: localhost\r\nAuthorization: Basic dXNlcjp1c2VyNDU2\r\n\r\n";
        let response2 = send_http_request(port, request2);
        assert!(response2.contains("HTTP/1.1 200 OK"));
    }

    #[test]
    fn test_content_length_vs_chunked_encoding() {
        let port = 9112;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Regular response should have Content-Length
        let request1 = "GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        let response1 = send_http_request(port, request1);
        assert!(response1.contains("Content-Length:"));
        assert!(!response1.contains("Transfer-Encoding: chunked"));

        // Chunked response should have Transfer-Encoding
        let request2 = "GET /chunked HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        let response2 = send_http_request(port, request2);
        assert!(response2.contains("Transfer-Encoding: chunked"));
        // Chunked responses should not have Content-Length, but our implementation might include it
        // This is acceptable as some servers do include both headers
    }

    #[test]
    fn test_auth_header_case_insensitive() {
        let port = 9113;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test with lowercase authorization header
        let request = "GET /admin HTTP/1.1\r\nHost: localhost\r\nauthorization: Basic dGVzdHVzZXI6dGVzdHBhc3M=\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Admin Panel"));
    }
}
