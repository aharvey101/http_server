use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

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
    
    // Fill salt with time-based pseudo-random values
    for (i, byte) in salt.iter_mut().enumerate() {
        *byte = ((time.wrapping_mul(31).wrapping_add(i as u64)) % 256) as u8;
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
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Helper function to decode hex string to bytes
fn hex_decode(hex_str: &str) -> Result<Vec<u8>, &'static str> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test_password123";
        let salt = generate_salt();
        let hash1 = hash_password(password, &salt);
        let hash2 = hash_password(password, &salt);
        
        // Same password with same salt should produce same hash
        assert_eq!(hash1, hash2);
        
        // Verify should work correctly
        assert!(verify_password(password, &hash1));
        assert!(!verify_password("wrong_password", &hash1));
    }

    #[test]
    fn test_different_salts_produce_different_hashes() {
        let password = "same_password";
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        let hash1 = hash_password(password, &salt1);
        let hash2 = hash_password(password, &salt2);
        
        // Different salts should produce different hashes
        assert_ne!(hash1, hash2);
        
        // But both should verify correctly
        assert!(verify_password(password, &hash1));
        assert!(verify_password(password, &hash2));
    }

    #[test]
    fn test_hex_encoding_decoding() {
        let original = b"hello world";
        let encoded = hex_encode(original);
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(original.to_vec(), decoded);
    }
}
