use super::helpers::*;

#[cfg(test)]
mod tests {
    use super::*;

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
}
