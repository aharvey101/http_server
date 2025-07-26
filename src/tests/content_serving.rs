use super::helpers::*;

// =======================
// STEP 7: CONTENT SERVING TESTS
// =======================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_file_serving_index() {
        let port = 9001;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /static/index.html HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/html"));
        // Should contain some content from the index.html file
        assert!(!response.is_empty());
    }

    #[test]
    fn test_static_file_serving_css() {
        let port = 9002;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /static/assets/style.css HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/css"));
    }

    #[test]
    fn test_static_file_serving_javascript() {
        let port = 9003;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /static/assets/script.js HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: application/javascript"));
    }

    #[test]
    fn test_static_file_serving_text() {
        let port = 9004;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /static/assets/readme.txt HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/plain"));
    }

    #[test]
    fn test_directory_listing() {
        let port = 9005;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /static/ HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/html"));
        assert!(response.contains("Directory Listing"));
        assert!(response.contains("index.html"));
        assert!(response.contains("about.html"));
        assert!(response.contains("assets/"));
        // Should contain directory listing styling
        assert!(response.contains("📁"));
        assert!(response.contains("📄"));
    }

    #[test]
    fn test_directory_listing_assets() {
        let port = 9006;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /static/assets/ HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/html"));
        assert!(response.contains("Directory Listing"));
        assert!(response.contains("readme.txt"));
        assert!(response.contains("script.js"));
        assert!(response.contains("style.css"));
        // Should contain parent directory link
        assert!(response.contains("Parent Directory"));
    }

    #[test]
    fn test_static_file_not_found() {
        let port = 9007;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let request = "GET /static/nonexistent.html HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = send_http_request(port, request);

        assert!(response.contains("HTTP/1.1 404 Not Found"));
        assert!(response.contains("404 - Page Not Found"));
    }

    #[test]
    fn test_mime_type_detection() {
        let port = 9009;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test various file types and their MIME types
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
}
