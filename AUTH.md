# HTTP Server Authentication System

This HTTP server now uses secure password hashing instead of storing passwords in plain text.

## Password Hashing

The server uses SHA256 with random salts to hash passwords. Each password is hashed with a unique 16-byte salt, ensuring that:

- Even identical passwords have different hashes
- Passwords cannot be easily reversed
- Rainbow table attacks are prevented

## Adding Users

### Method 1: Using the password hashing utility

Generate a hashed password:
```bash
cargo run --bin hash_password mypassword123
```

This will output something like:
```
Hashed password: 1c7c2f659cd7518ce8e5468ea6af27de:dbddd0472562aebfa60ca22e92e86582fa6de5038d2bf54e77662dc1d128476e
```

Then add this hash to your configuration or code:
```rust
server.add_auth_user("username", "1c7c2f659cd7518ce8e5468ea6af27de:dbddd0472562aebfa60ca22e92e86582fa6de5038d2bf54e77662dc1d128476e");
```

### Method 2: Using the convenience method (for development/testing)

```rust
server.add_auth_user_with_password("username", "plaintext_password");
```

This method automatically generates a salt and hashes the password for you.

## Configuration

The default configuration includes two users with hashed passwords:
- `admin` with password `password123`
- `user` with password `secret`

These passwords are automatically hashed when the default configuration is created.

## API Methods

### HttpServer Methods

- `add_auth_user(username: &str, hashed_password: &str)` - Adds a user with a pre-hashed password
- `add_auth_user_with_password(username: &str, plain_password: &str)` - Adds a user and hashes the password automatically
- `add_protected_path(path: &str)` - Marks a path as requiring authentication

### Auth Module Functions

- `hash_password(password: &str, salt: &[u8]) -> String` - Hashes a password with a given salt
- `verify_password(password: &str, stored_hash: &str) -> bool` - Verifies a password against a stored hash
- `generate_salt() -> [u8; 16]` - Generates a random 16-byte salt

## Security Features

1. **Salted Hashing**: Each password uses a unique random salt
2. **SHA256**: Industry-standard cryptographic hash function
3. **No Plain Text Storage**: Passwords are never stored in plain text
4. **Constant-Time Comparison**: Password verification uses secure comparison methods

## Migration from Plain Text

If you have existing plain text passwords in your configuration, you need to:

1. Generate hashed versions using the `hash_password` utility
2. Update your configuration to use the hashed passwords
3. Use `add_auth_user()` instead of `add_auth_user_with_password()` if you're manually managing passwords

## Example Usage

```rust
use api::HttpServer;

let mut server = HttpServer::new("127.0.0.1:8080").unwrap();

// Add users with automatic password hashing
server.add_auth_user_with_password("admin", "secure_password");
server.add_auth_user_with_password("user", "another_password");

// Protect admin paths
server.add_protected_path("/admin");
server.add_protected_path("/api/admin");

server.start().unwrap();
```

The server will now require HTTP Basic Authentication for protected paths, and all passwords are securely hashed.
