use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

/// Structure to hold user authentication data
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub username: String,
    pub password_hash: String,
}

/// Structure to hold session token data
#[derive(Clone, Debug)]
pub struct AuthToken {
    pub token: String,
    pub username: String,
    pub expires_at: u64, // Unix timestamp
}

/// Structure for managing authentication tokens
pub struct TokenManager {
    tokens: std::sync::Mutex<std::collections::HashMap<String, AuthToken>>,
}

impl TokenManager {
    pub fn new() -> Self {
        TokenManager {
            tokens: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Generate a new token for a user
    pub fn generate_token(&self, username: &str) -> String {
        let token = generate_token();
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600; // Token expires in 1 hour
        
        let auth_token = AuthToken {
            token: token.clone(),
            username: username.to_string(),
            expires_at,
        };
        
        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.insert(token.clone(), auth_token);
        }
        token
    }

    /// Validate a token and return the username if valid
    pub fn validate_token(&self, token: &str) -> Option<String> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Ok(mut tokens) = self.tokens.lock() {
            if let Some(auth_token) = tokens.get(token) {
                if auth_token.expires_at > current_time {
                    return Some(auth_token.username.clone());
                } else {
                    // Token expired, remove it
                    tokens.remove(token);
                }
            }
        }
        None
    }

    /// Revoke a token (logout)
    pub fn revoke_token(&self, token: &str) -> bool {
        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.remove(token).is_some()
        } else {
            false
        }
    }

    /// Clean up expired tokens
    pub fn cleanup_expired_tokens(&self) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.retain(|_, auth_token| auth_token.expires_at > current_time);
        }
    }
}

/// Generate a random token
pub fn generate_token() -> String {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    let mut hasher = DefaultHasher::new();
    time.hash(&mut hasher);
    
    // Add some additional entropy
    for i in 0..16 {
        (time.wrapping_mul(31).wrapping_add(i)).hash(&mut hasher);
    }
    
    let token_hash = hasher.finish();
    format!("{:016x}{:016x}", token_hash, time)
}

// Simple base64 decoder for authentication (simplified implementation)
pub fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();
    let input = input.trim();
    
    if input.len() % 4 != 0 {
        return Err("Invalid base64 length");
    }
    
    for chunk in input.as_bytes().chunks(4) {
        let mut values = [0u8; 4];
        
        for (i, &byte) in chunk.iter().enumerate() {
            if byte == b'=' {
                values[i] = 0;
            } else if let Some(pos) = CHARS.iter().position(|&c| c == byte) {
                values[i] = pos as u8;
            } else {
                return Err("Invalid base64 character");
            }
        }
        
        let combined = ((values[0] as u32) << 18) | 
                      ((values[1] as u32) << 12) | 
                      ((values[2] as u32) << 6) | 
                      (values[3] as u32);
        
        result.push((combined >> 16) as u8);
        if chunk[2] != b'=' {
            result.push((combined >> 8) as u8);
        }
        if chunk[3] != b'=' {
            result.push(combined as u8);
        }
    }
    
    Ok(result)
}

/// Generate a random salt for password hashing
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    // Use current time and a simple counter for pseudo-randomness
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Add a static counter to ensure uniqueness even for rapid calls
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
    
    // Fill salt with time-based and counter-based pseudo-random values
    for (i, byte) in salt.iter_mut().enumerate() {
        *byte = ((time.wrapping_mul(31).wrapping_add(counter).wrapping_add(i as u64)) % 256) as u8;
    }
    salt
}

/// Hash a password with a salt using DefaultHasher
pub fn hash_password(password: &str, salt: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    salt.hash(&mut hasher);
    password.hash(&mut hasher);
    let result = hasher.finish();
    
    // Convert salt and hash to hex strings and combine them
    let salt_hex = hex_encode(salt);
    let hash_hex = format!("{:016x}", result);
    format!("{}:{}", salt_hex, hash_hex)
}

/// Verify a password against a stored hash
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    if let Some((salt_hex, hash_hex)) = stored_hash.split_once(':') {
        if let Ok(salt) = hex_decode(salt_hex) {
            let mut hasher = DefaultHasher::new();
            salt.hash(&mut hasher);
            password.hash(&mut hasher);
            let actual_hash = hasher.finish();
            let actual_hash_hex = format!("{:016x}", actual_hash);
            
            return actual_hash_hex == hash_hex;
        }
    }
    false
}

/// Helper function to encode bytes as hex string
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Helper function to decode hex string to bytes
pub fn hex_decode(hex_str: &str) -> Result<Vec<u8>, &'static str> {
    if hex_str.len() % 2 != 0 {
        return Err("Invalid hex string length");
    }
    
    let mut result = Vec::new();
    for chunk in hex_str.as_bytes().chunks(2) {
        let hex_byte = std::str::from_utf8(chunk).map_err(|_| "Invalid UTF-8 in hex string")?;
        let byte = u8::from_str_radix(hex_byte, 16).map_err(|_| "Invalid hex character")?;
        result.push(byte);
    }
    Ok(result)
}

/// Simple JSON parsing for login requests (no external dependencies)
pub fn parse_login_request(json_body: &str) -> Option<(String, String)> {
    // Very simple JSON parsing - looks for "username" and "password" fields
    let mut username = None;
    let mut password = None;
    
    // Remove whitespace and braces
    let cleaned = json_body.trim().trim_start_matches('{').trim_end_matches('}');
    
    // Split by commas and parse each field
    for field in cleaned.split(',') {
        let field = field.trim();
        if let Some(colon_pos) = field.find(':') {
            let key = field[..colon_pos].trim().trim_matches('"');
            let value = field[colon_pos + 1..].trim().trim_matches('"');
            
            match key {
                "username" => username = Some(value.to_string()),
                "password" => password = Some(value.to_string()),
                _ => {}
            }
        }
    }
    
    if let (Some(u), Some(p)) = (username, password) {
        Some((u, p))
    } else {
        None
    }
}

/// Generate JSON response for successful login
pub fn create_login_response(token: &str) -> String {
    format!(r#"{{"success": true, "token": "{}"}}"#, token)
}

/// Generate JSON response for errors
pub fn create_error_response(message: &str) -> String {
    format!(r#"{{"success": false, "error": "{}"}}"#, message)
}
