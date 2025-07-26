use std::collections::HashMap;

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl HttpRequest {
    pub fn parse(request_data: &str) -> Result<Self, &'static str> {
        let lines: Vec<&str> = request_data.lines().collect();
        
        if lines.is_empty() {
            return Err("Empty request");
        }

        // Parse HTTP request line (method, path, version)
        let request_line_parts: Vec<&str> = lines[0].split_whitespace().collect();
        if request_line_parts.len() != 3 {
            return Err("Invalid request line");
        }

        let method = request_line_parts[0].to_string();
        let path = request_line_parts[1].to_string();
        let version = request_line_parts[2].to_string();

        // Parse HTTP headers (split by lines)
        let mut headers = HashMap::new();
        let mut header_end_index = 1;

        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.is_empty() {
                header_end_index = i;
                break;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_lowercase();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Extract request body if present
        let body = if header_end_index + 1 < lines.len() {
            lines[header_end_index + 1..].join("\n")
        } else {
            String::new()
        };

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body,
        })
    }
}
