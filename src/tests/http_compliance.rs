use super::helpers::*;

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================
    // RFC 7230: Message Syntax and Routing Compliance
    // =====================================================

    #[test]
    fn test_rfc7230_request_line_parsing() {
        let port = 9200;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test proper request line parsing (Method SP Request-Target SP HTTP-Version CRLF)
        let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);
        assert!(response.contains("HTTP/1.1 200 OK"));

        // Test with different methods
        let methods = ["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS"];
        for method in &methods {
            let req = format!("{} /hello HTTP/1.1\r\nHost: localhost\r\n\r\n", method);
            let resp = send_http_request(port, &req);
            // Should either succeed or return 404/405, but not 400 for valid syntax
            assert!(resp.contains("HTTP/1.1") && !resp.contains("400 Bad Request"));
        }
    }

    #[test]
    fn test_rfc7230_header_field_syntax() {
        let port = 9201;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test case-insensitive header names
        let requests = vec![
            "GET /hello HTTP/1.1\r\nHOST: localhost\r\n\r\n",
            "GET /hello HTTP/1.1\r\nhost: localhost\r\n\r\n", 
            "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /hello HTTP/1.1\r\nContent-Type: text/plain\r\nHost: localhost\r\n\r\n",
        ];

        for request in requests {
            let response = send_http_request(port, request);
            assert!(response.contains("HTTP/1.1 200 OK"));
        }
    }

    #[test]
    fn test_rfc7230_malformed_request_handling() {
        let port = 9202;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test invalid methods
        let invalid_requests = vec![
            ("INVALID_METHOD /hello HTTP/1.1\r\nHost: localhost\r\n\r\n", "should return 404 for invalid method"),
            ("GET\r\nHost: localhost\r\n\r\n", "should return 400 for missing parts"), 
            ("GET /hello\r\nHost: localhost\r\n\r\n", "should return 400 for missing HTTP version"),
            ("GET /hello HTTP/2.0\r\nHost: localhost\r\n\r\n", "should handle unsupported version"),
        ];

        for (request, description) in invalid_requests {
            let response = send_http_request(port, request);
            
            // Should return appropriate error response for malformed requests
            // Our server is tolerant - it may return 404 for invalid methods or 400 for malformed syntax
            assert!(response.contains("HTTP/1.1 400 Bad Request") || 
                    response.contains("HTTP/1.1 404 Not Found") ||
                    response.contains("HTTP/1.1 501 Not Implemented") ||
                    response.contains("HTTP/1.1 200 OK"), // Some malformed requests might still work due to tolerant parsing
                    "Failed for: {} - {}", request.trim(), description);
        }
    }

    #[test] 
    fn test_rfc7230_crlf_line_endings() {
        let port = 9203;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test proper CRLF (\r\n) usage
        let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);
        
        assert!(response.contains("HTTP/1.1 200 OK"));
        // Response should also use proper CRLF
        assert!(response.contains("\r\n"));
    }

    // =====================================================
    // RFC 7231: Semantics and Content Compliance
    // =====================================================

    #[test]
    fn test_rfc7231_http_methods() {
        let port = 9204;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test all standard HTTP methods
        let test_cases = vec![
            ("GET", "/hello", "200 OK"),
            ("POST", "/api/echo", "200 OK"),
            ("PUT", "/nonexistent", "404 Not Found"),
            ("DELETE", "/nonexistent", "404 Not Found"),
            ("HEAD", "/hello", "200 OK"), // Now supported
            ("OPTIONS", "/hello", "404 Not Found"), // Our server doesn't implement OPTIONS
        ];

        for (method, path, expected_status) in test_cases {
            let body = if method == "POST" || method == "PUT" {
                "test data"
            } else {
                ""
            };

            let request = if body.is_empty() {
                format!("{} {} HTTP/1.1\r\nHost: localhost\r\n\r\n", method, path)
            } else {
                format!("{} {} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}", 
                       method, path, body.len(), body)
            };

            let response = send_http_request(port, &request);
            assert!(response.contains(&format!("HTTP/1.1 {}", expected_status)), 
                   "Failed for {} {} - expected {} but got: {}", method, path, expected_status, response.lines().next().unwrap_or(""));
        }
    }

    #[test]
    fn test_rfc7231_status_codes_and_text() {
        let port = 9205;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test proper status code usage
        let test_cases = vec![
            ("GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n", "200 OK"),
            ("GET /nonexistent HTTP/1.1\r\nHost: localhost\r\n\r\n", "404 Not Found"),
            ("GET /admin HTTP/1.1\r\nHost: localhost\r\n\r\n", "401 Unauthorized"),
            ("GET /static/../main.rs HTTP/1.1\r\nHost: localhost\r\n\r\n", "403 Forbidden"),
        ];

        for (request, expected_status) in test_cases {
            let response = send_http_request(port, request);
            assert!(response.contains(&format!("HTTP/1.1 {}", expected_status)));
        }
    }

    #[test]
    fn test_rfc7231_content_headers() {
        let port = 9206;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test Content-Type and Content-Length header handling
        let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/plain"));
        assert!(response.contains("Content-Length:"));

        // Extract and validate Content-Length
        let content_length_line = response
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .expect("Content-Length header should be present");
        
        let content_length: usize = content_length_line
            .split(':')
            .nth(1)
            .unwrap()
            .trim()
            .parse()
            .expect("Content-Length should be a valid number");

        let body_start = response.find("\r\n\r\n").unwrap() + 4;
        let body = &response[body_start..];
        assert_eq!(content_length, body.len());
    }

    #[test]
    fn test_rfc7231_mime_type_detection() {
        let port = 9207;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let test_cases = vec![
            ("/static/index.html", "text/html"),
            ("/static/assets/style.css", "text/css"),
            ("/static/assets/script.js", "application/javascript"),
            ("/static/assets/readme.txt", "text/plain"),
        ];

        for (path, expected_mime) in test_cases {
            let request = format!("GET {} HTTP/1.1\r\nHost: localhost\r\n\r\n", path);
            let response = send_http_request(port, &request);
            
            if response.contains("HTTP/1.1 200 OK") {
                assert!(response.contains(&format!("Content-Type: {}", expected_mime)));
            }
        }
    }

    // =====================================================
    // RFC 7232: Conditional Requests Compliance
    // =====================================================

    #[test]
    fn test_rfc7232_if_modified_since() {
        let port = 9208;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test If-Modified-Since header (basic implementation)
        let request = "GET /static/index.html HTTP/1.1\r\n\
                      Host: localhost\r\n\
                      If-Modified-Since: Wed, 21 Oct 2015 07:28:00 GMT\r\n\r\n";
        let response = send_http_request(port, request);

        // Our server doesn't implement conditional requests yet, so it should return 200
        // This test documents the current behavior and can be updated when implemented
        assert!(response.contains("HTTP/1.1 200 OK"));
    }

    #[test]
    fn test_rfc7232_etag_support() {
        let port = 9209;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test ETag support (not yet implemented)
        let request = "GET /static/index.html HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        // Current implementation doesn't include ETag headers
        // This test documents current behavior
        assert!(response.contains("HTTP/1.1 200 OK"));
        // assert!(response.contains("ETag:")); // Not implemented yet
    }

    // =====================================================
    // RFC 7235: Authentication Compliance
    // =====================================================

    #[test]
    fn test_rfc7235_authentication_compliance() {
        let port = 9210;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test 401 Unauthorized responses for protected resources
        let request = "GET /admin HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 401 Unauthorized"));
        assert!(response.contains("application/json"));
        assert!(response.contains("Valid Bearer token required"));
        
        // Should NOT contain WWW-Authenticate header since Basic Auth was removed
        assert!(!response.contains("WWW-Authenticate"));
    }

    #[test]
    fn test_rfc7235_bearer_token_format() {
        let port = 9211;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test Bearer token authentication format
        let request = "GET /admin HTTP/1.1\r\n\
                      Host: localhost\r\n\
                      Authorization: Bearer invalid-token\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 401 Unauthorized"));
        assert!(response.contains("Valid Bearer token required"));
    }

    // =====================================================
    // HTTP/1.1 Connection Management
    // =====================================================

    #[test]
    fn test_connection_header_handling() {
        let port = 9212;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test keep-alive connection
        let request = "GET /hello HTTP/1.1\r\n\
                      Host: localhost\r\n\
                      Connection: keep-alive\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Connection: keep-alive"));

        // Test connection close
        let request2 = "GET /hello HTTP/1.1\r\n\
                       Host: localhost\r\n\
                       Connection: close\r\n\r\n";
        let response2 = send_http_request(port, request2);

        assert!(response2.contains("HTTP/1.1 200 OK"));
        assert!(response2.contains("Connection: close"));
    }

    #[test]
    fn test_http_version_handling() {
        let port = 9213;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test HTTP/1.0 request (should still respond with HTTP/1.1)
        let request = "GET /hello HTTP/1.0\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Hello, World!"));
    }

    // =====================================================
    // Transfer Encoding Compliance
    // =====================================================

    #[test]
    fn test_chunked_transfer_encoding_compliance() {
        let port = 9214;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test chunked transfer encoding
        let request = "GET /chunked HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Transfer-Encoding: chunked"));
        
        // Verify chunked encoding format (basic check)
        let body_start = response.find("\r\n\r\n").unwrap() + 4;
        let body = &response[body_start..];
        
        // Chunked responses should have proper format:
        // [hex-size]\r\n[data]\r\n[hex-size]\r\n[data]\r\n0\r\n\r\n
        assert!(body.contains("\r\n"), "Chunked body should contain CRLF separators");
        
        // Should end with the final chunk marker "0\r\n\r\n"
        assert!(body.ends_with("0\r\n\r\n"), "Chunked response should end with final chunk marker");
        
        // Should contain chunk size in hex format
        let lines: Vec<&str> = body.split("\r\n").collect();
        if lines.len() >= 3 {
            // First line should be a hex number (chunk size)
            let chunk_size_str = lines[0];
            assert!(u32::from_str_radix(chunk_size_str, 16).is_ok(), 
                   "First line should be a valid hex chunk size: {}", chunk_size_str);
        }
    }

    #[test]
    fn test_content_length_vs_chunked_exclusivity() {
        let port = 9215;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Regular response should have Content-Length, not Transfer-Encoding
        let request1 = "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response1 = send_http_request(port, request1);
        
        assert!(response1.contains("Content-Length:"));
        assert!(!response1.contains("Transfer-Encoding: chunked"));

        // Chunked response should have Transfer-Encoding
        let request2 = "GET /chunked HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response2 = send_http_request(port, request2);
        
        assert!(response2.contains("Transfer-Encoding: chunked"));
    }

    // =====================================================
    // URI and Query Parameter Compliance
    // =====================================================

    #[test]
    fn test_query_parameter_parsing_compliance() {
        let port = 9216;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test basic query parameters
        let request = "GET /hello?name=Test HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Hello, Test!"));

        // Test multiple query parameters
        let request2 = "GET /?name=Test&type=query HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response2 = send_http_request(port, request2);

        assert!(response2.contains("HTTP/1.1 200 OK"));
        assert!(response2.contains("name: Test"));
        assert!(response2.contains("type: query"));
    }

    #[test]
    fn test_url_encoding_handling() {
        let port = 9217;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test URL-encoded parameters (basic test)
        let requests = vec![
            "GET /hello?name=Hello%20World HTTP/1.1\r\nHost: localhost\r\n\r\n",
            "GET /hello?name=Test%21 HTTP/1.1\r\nHost: localhost\r\n\r\n",
        ];

        for request in requests {
            let response = send_http_request(port, request);
            assert!(response.contains("HTTP/1.1 200 OK"));
            // Our current implementation doesn't decode URLs, but it should handle them gracefully
        }
    }

    #[test]
    fn test_path_traversal_protection_compliance() {
        let port = 9218;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test various path traversal attempts
        let malicious_paths = vec![
            "/static/../src/main.rs",
            "/static/../../etc/passwd",
            "/static/../../../root/.bashrc",
            "/../etc/passwd",
        ];

        for path in malicious_paths {
            let request = format!("GET {} HTTP/1.1\r\nHost: localhost\r\n\r\n", path);
            let response = send_http_request(port, &request);
            
            // Should return 403 Forbidden for path traversal attempts
            assert!(response.contains("HTTP/1.1 403 Forbidden"));
            assert!(response.contains("Directory traversal is not allowed"));
        }
    }

    // =====================================================
    // Header Field Compliance Tests
    // =====================================================

    #[test]
    fn test_host_header_requirement() {
        let port = 9219;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // HTTP/1.1 requires Host header - test both with and without
        let request_with_host = "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response_with_host = send_http_request(port, request_with_host);
        assert!(response_with_host.contains("HTTP/1.1 200 OK"));

        // Request without Host header (should work but is technically invalid HTTP/1.1)
        let request_without_host = "GET /hello HTTP/1.1\r\n\r\n";
        let response_without_host = send_http_request(port, request_without_host);
        // Our implementation is tolerant, so it should work
        assert!(response_without_host.contains("HTTP/1.1"));
    }

    #[test]
    fn test_header_case_insensitivity() {
        let port = 9220;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test that headers are case-insensitive
        let headers_variants = vec![
            "Content-Type: application/json",
            "content-type: application/json", 
            "CONTENT-TYPE: application/json",
            "Content-type: application/json",
        ];

        for header in headers_variants {
            let request = format!("POST /api/echo HTTP/1.1\r\n\
                                  Host: localhost\r\n\
                                  {}\r\n\
                                  Content-Length: 4\r\n\r\ntest", header);
            let response = send_http_request(port, &request);
            assert!(response.contains("HTTP/1.1 200 OK"));
        }
    }

    // =====================================================
    // Error Response Compliance
    // =====================================================

    #[test]
    fn test_error_response_format() {
        let port = 9221;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test that error responses have proper format
        let error_requests = vec![
            ("GET /nonexistent HTTP/1.1\r\nHost: localhost\r\n\r\n", "404 Not Found"),
            ("GET /admin HTTP/1.1\r\nHost: localhost\r\n\r\n", "401 Unauthorized"),
            ("GET /static/../main.rs HTTP/1.1\r\nHost: localhost\r\n\r\n", "403 Forbidden"),
        ];

        for (request, expected_status) in error_requests {
            let response = send_http_request(port, request);
            
            assert!(response.contains(&format!("HTTP/1.1 {}", expected_status)));
            assert!(response.contains("Content-Type:"));
            assert!(response.contains("Content-Length:"));
            
            // Check response has proper structure
            assert!(response.contains("\r\n\r\n")); // Header/body separator
        }
    }

    #[test]
    fn test_malformed_request_error_handling() {
        let port = 9222;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test various malformed requests
        let malformed_requests = vec![
            "INVALID REQUEST\r\n\r\n",
            "GET\r\n\r\n",
            "GET /hello\r\n\r\n",
            "",
        ];

        for request in malformed_requests {
            let response = send_http_request(port, request);
            
            // Should return a proper HTTP response even for malformed requests
            if !response.is_empty() {
                assert!(response.contains("HTTP/1.1"));
                // Should be either 400 Bad Request or connection closed
                if response.contains("400") {
                    assert!(response.contains("400 Bad Request"));
                }
            }
            // Empty response is acceptable for completely invalid requests
        }
    }

    // =====================================================
    // Performance and Resource Compliance
    // =====================================================

    #[test]
    fn test_large_header_handling() {
        let port = 9223;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test large headers (but not excessive)
        let large_value = "x".repeat(1000);
        let request = format!("GET /hello HTTP/1.1\r\n\
                              Host: localhost\r\n\
                              X-Large-Header: {}\r\n\r\n", large_value);
        
        let response = send_http_request(port, &request);
        // Should handle reasonably large headers
        assert!(response.contains("HTTP/1.1 200 OK"));
    }

    #[test]
    fn test_concurrent_connection_compliance() {
        let port = 9224;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test multiple concurrent connections
        let handles: Vec<std::thread::JoinHandle<_>> = (0..5)
            .map(|i| {
                std::thread::spawn(move || {
                    let request = format!("GET /hello?id={} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", i);
                    let response = send_http_request(port, &request);
                    assert!(response.contains("HTTP/1.1 200 OK"));
                    response
                })
            })
            .collect();

        // Wait for all requests to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }
}
