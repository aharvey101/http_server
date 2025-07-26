use super::helpers::*;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::{Arc, Barrier};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_threading_concurrent_requests() {
        let port = 9100;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let num_threads = 10;
        let requests_per_thread = 5;
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = Vec::new();

        // Spawn multiple threads that will all make requests simultaneously
        for thread_id in 0..num_threads {
            let barrier = Arc::clone(&barrier);
            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();
                
                let mut successful_requests = 0;
                for request_id in 0..requests_per_thread {
                    let request = format!("GET /hello?thread={}&request={} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", thread_id, request_id);
                    
                    match std::panic::catch_unwind(|| {
                        send_http_request(port, &request)
                    }) {
                        Ok(response) => {
                            if response.contains("HTTP/1.1 200 OK") && response.contains("Hello, World!") {
                                successful_requests += 1;
                            }
                        }
                        Err(_) => {
                            // Request failed, but we'll count successful ones
                        }
                    }
                }
                successful_requests
            });
            handles.push(handle);
        }

        // Collect results from all threads
        let mut total_successful = 0;
        let mut total_expected = 0;
        for handle in handles {
            let successful = handle.join().unwrap();
            total_successful += successful;
            total_expected += requests_per_thread;
        }

        // We expect at least 80% success rate for concurrent requests
        let success_rate = (total_successful as f64) / (total_expected as f64);
        assert!(success_rate >= 0.8, 
                "Success rate too low: {}/{} = {:.2}%", 
                total_successful, total_expected, success_rate * 100.0);
        
        println!("Multi-threading test: {}/{} requests successful ({:.1}%)", 
                total_successful, total_expected, success_rate * 100.0);
    }

    #[test]
    fn test_connection_pooling_reuse() {
        let port = 9101;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test connection reuse with keep-alive
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

        // Send multiple requests on the same connection
        for i in 0..3 {
            let request = format!("GET /hello?request={} HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n", i);
            
            // Send request
            stream.write_all(request.as_bytes()).unwrap();
            
            // Read response
            let mut buffer = [0; 2048];
            let bytes_read = stream.read(&mut buffer).unwrap();
            let response = String::from_utf8_lossy(&buffer[..bytes_read]);
            
            // Verify response
            assert!(response.contains("HTTP/1.1 200 OK"));
            assert!(response.contains("Hello, World!"));
            
            // Should indicate connection will be kept alive
            assert!(response.contains("Connection: keep-alive") || 
                   !response.contains("Connection: close"));
        }

        println!("Connection pooling test: Successfully reused connection for multiple requests");
    }

    #[test]
    fn test_buffered_stream_performance() {
        let port = 9102;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Test with a reasonably large request body
        let large_body = "x".repeat(1024); // 1KB body
        let request = format!(
            "POST /api/echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            large_body.len(),
            large_body
        );

        let start_time = Instant::now();
        let response = send_http_request(port, &request);
        let duration = start_time.elapsed();

        // Verify the response is correct
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains(&large_body)); // Echo should return the body

        // Performance check - should complete reasonably quickly
        assert!(duration < Duration::from_secs(2), 
               "Buffered request took too long: {:?}", duration);
        
        println!("Buffered stream test: 1KB request completed in {:?}", duration);
    }

    #[test]
    fn test_memory_usage_large_concurrent_load() {
        let port = 9103;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        let num_concurrent = 20;
        let barrier = Arc::new(Barrier::new(num_concurrent));
        let mut handles = Vec::new();

        let start_time = Instant::now();

        // Spawn many concurrent connections
        for i in 0..num_concurrent {
            let barrier = Arc::clone(&barrier);
            let handle = thread::spawn(move || {
                barrier.wait();
                
                // Each thread makes a request with some data
                let body = format!("Request from thread {} with data: {}", i, "test".repeat(100));
                let request = format!(
                    "POST /api/echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                
                match std::panic::catch_unwind(|| {
                    send_http_request(port, &request)
                }) {
                    Ok(response) => {
                        response.contains("HTTP/1.1 200 OK")
                    }
                    Err(_) => false
                }
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        let mut successful = 0;
        for handle in handles {
            if handle.join().unwrap() {
                successful += 1;
            }
        }

        let total_time = start_time.elapsed();

        // Check that most requests succeeded
        let success_rate = (successful as f64) / (num_concurrent as f64);
        assert!(success_rate >= 0.7, 
               "Too many requests failed under load: {}/{}", successful, num_concurrent);

        // Check that it completed in reasonable time (not too slow due to memory issues)
        assert!(total_time < Duration::from_secs(10), 
               "Load test took too long: {:?}", total_time);

        println!("Memory usage test: {}/{} requests successful in {:?}", 
                successful, num_concurrent, total_time);
    }

    #[test]
    fn test_thread_pool_stress() {
        let port = 9104;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Rapidly fire many requests to stress the thread pool
        let num_requests = 50;
        let mut handles = Vec::new();

        let start_time = Instant::now();

        for i in 0..num_requests {
            let handle = thread::spawn(move || {
                let request = format!("GET /hello?id={} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", i);
                
                match std::panic::catch_unwind(|| {
                    send_http_request(port, &request)
                }) {
                    Ok(response) => {
                        response.contains("HTTP/1.1 200 OK") && response.contains("Hello, World!")
                    }
                    Err(_) => false
                }
            });
            handles.push(handle);
        }

        // Collect results
        let mut successful = 0;
        for handle in handles {
            if handle.join().unwrap() {
                successful += 1;
            }
        }

        let total_time = start_time.elapsed();
        let success_rate = (successful as f64) / (num_requests as f64);

        // Thread pool should handle most requests successfully
        assert!(success_rate >= 0.8, 
               "Thread pool stress test failed: {}/{} requests successful", 
               successful, num_requests);

        // Should complete reasonably quickly
        assert!(total_time < Duration::from_secs(15), 
               "Thread pool stress test took too long: {:?}", total_time);

        println!("Thread pool stress test: {}/{} requests successful in {:?} ({:.1}% success rate)", 
                successful, num_requests, total_time, success_rate * 100.0);
    }

    #[test]
    fn test_performance_baseline() {
        let port = 9105;
        let _server_handle = start_test_server(port);
        wait_for_server(port);

        // Measure baseline performance for simple requests
        let num_requests = 10;
        let mut total_time = Duration::new(0, 0);

        for _ in 0..num_requests {
            let start = Instant::now();
            let request = "GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
            let response = send_http_request(port, request);
            let duration = start.elapsed();

            assert!(response.contains("HTTP/1.1 200 OK"));
            total_time += duration;
        }

        let average_time = total_time / num_requests as u32;
        
        // Each simple request should complete in reasonable time
        assert!(average_time < Duration::from_millis(100), 
               "Average request time too slow: {:?}", average_time);

        println!("Performance baseline: Average request time: {:?}", average_time);
    }
}
