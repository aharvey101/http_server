#[cfg(test)]
mod tests {
    use api::{
        generate_salt, hash_password, verify_password, hex_encode, hex_decode,
        TokenManager, parse_login_request
    };

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

    #[test]
    fn test_token_generation_and_validation() {
        let token_manager = TokenManager::new();
        let username = "testuser";
        
        // Generate a token
        let token = token_manager.generate_token(username);
        assert!(!token.is_empty());
        
        // Validate the token
        let validated_username = token_manager.validate_token(&token);
        assert_eq!(validated_username, Some(username.to_string()));
        
        // Invalid token should return None
        assert_eq!(token_manager.validate_token("invalid_token"), None);
    }

    #[test]
    fn test_token_revocation() {
        let token_manager = TokenManager::new();
        let username = "testuser";
        
        let token = token_manager.generate_token(username);
        assert!(token_manager.validate_token(&token).is_some());
        
        // Revoke the token
        assert!(token_manager.revoke_token(&token));
        assert!(token_manager.validate_token(&token).is_none());
        
        // Revoking again should return false
        assert!(!token_manager.revoke_token(&token));
    }

    #[test]
    fn test_json_parsing() {
        let json = r#"{"username": "testuser", "password": "testpass"}"#;
        let (username, password) = parse_login_request(json).unwrap();
        assert_eq!(username, "testuser");
        assert_eq!(password, "testpass");
        
        // Test with different order
        let json2 = r#"{"password": "pass123", "username": "user123"}"#;
        let (username2, password2) = parse_login_request(json2).unwrap();
        assert_eq!(username2, "user123");
        assert_eq!(password2, "pass123");
        
        // Test invalid JSON
        let invalid_json = r#"{"username": "test"}"#; // missing password
        assert!(parse_login_request(invalid_json).is_none());
    }
}
