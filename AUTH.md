# HTTP Server Authentication System

This HTTP server supports both traditional password hashing and modern token-based authentication.

## Authentication Methods

### 1. Basic Authentication (Username/Password)
Uses HTTP Basic Auth with salted password hashing for traditional authentication.

### 2. Token-Based Authentication (Recommended)
Users can register/login to receive a JWT-like token for subsequent requests.

## Password Hashing

The server uses Rust's DefaultHasher with random salts to hash passwords. Each password is hashed with a unique 16-byte salt, ensuring that:

- Even identical passwords have different hashes
- Passwords cannot be easily reversed
- Rainbow table attacks are prevented

## Token-Based Authentication

### Registration Endpoint: `POST /api/register`
Register a new user and receive an authentication token.

**Request:**
```json
{
  "username": "your_username",
  "password": "your_password"
}
```

**Response (201 Created):**
```json
{
  "success": true,
  "token": "abc123def456..."
}
```

**Error Response (409 Conflict - username exists):**
```json
{
  "success": false,
  "error": "Username already exists"
}
```

### Login Endpoint: `POST /api/login`
Login with existing credentials and receive an authentication token.

**Request:**
```json
{
  "username": "your_username",
  "password": "your_password"
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "token": "abc123def456..."
}
```

**Error Response (401 Unauthorized):**
```json
{
  "success": false,
  "error": "Invalid username or password"
}
```

### Logout Endpoint: `POST /api/logout`
Revoke an authentication token.

**Request Headers:**
```
Authorization: Bearer abc123def456...
```

**Response (200 OK):**
```json
{
  "success": true,
  "message": "Logged out successfully"
}
```

### Using Tokens for Protected Resources

For protected endpoints, include the token in the Authorization header:

```
Authorization: Bearer abc123def456...
```

## Token Management

- **Token Expiration**: Tokens expire after 1 hour
- **Automatic Cleanup**: Expired tokens are automatically removed
- **Thread Safety**: Token management is thread-safe using Mutex

## Adding Users (Legacy/Admin)

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

- `hash_password(password: &str, salt: &[u8]) -> String` - Hashes a password with a given salt using DefaultHasher
- `verify_password(password: &str, stored_hash: &str) -> bool` - Verifies a password against a stored hash
- `generate_salt() -> [u8; 16]` - Generates a pseudo-random 16-byte salt based on system time
- `generate_token() -> String` - Generates a unique authentication token
- `parse_login_request(json_body: &str) -> Option<(String, String)>` - Parses JSON login requests
- `create_login_response(token: &str) -> String` - Creates JSON response with token
- `create_error_response(message: &str) -> String` - Creates JSON error response

### TokenManager Methods

- `generate_token(username: &str) -> String` - Generate a new token for a user
- `validate_token(token: &str) -> Option<String>` - Validate token and return username
- `revoke_token(token: &str) -> bool` - Revoke a token (logout)
- `cleanup_expired_tokens()` - Remove expired tokens

## Security Features

1. **Salted Hashing**: Each password uses a unique random salt
2. **DefaultHasher**: Uses Rust's standard library hash function for password storage
3. **No Plain Text Storage**: Passwords are never stored in plain text
4. **Hex Encoding**: Salts and hashes are stored as hexadecimal strings
5. **Token Expiration**: Authentication tokens expire after 1 hour
6. **Thread Safety**: Both user storage and token management are thread-safe
7. **Multiple Auth Methods**: Supports both Basic Auth and Bearer Token authentication

## Migration from Plain Text

If you have existing plain text passwords in your configuration, you need to:

1. Generate hashed versions using the `hash_password` utility
2. Update your configuration to use the hashed passwords
3. Use `add_auth_user()` instead of `add_auth_user_with_password()` if you're manually managing passwords

## Example Usage

### Traditional Basic Auth
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

### Token-Based Auth (Client Usage)

**1. Register a new user:**
```bash
curl -X POST http://localhost:8080/api/register \
  -H "Content-Type: application/json" \
  -d '{"username": "newuser", "password": "securepass123"}'
```

**2. Login with existing user:**
```bash
curl -X POST http://localhost:8080/api/login \
  -H "Content-Type: application/json" \
  -d '{"username": "newuser", "password": "securepass123"}'
```

**3. Access protected resources:**
```bash
curl -X GET http://localhost:8080/api/protected \
  -H "Authorization: Bearer your_token_here"
```

**4. Logout (revoke token):**
```bash
curl -X POST http://localhost:8080/api/logout \
  -H "Authorization: Bearer your_token_here"
```

The server now supports both traditional username/password authentication and modern token-based authentication, allowing for flexible integration with web applications and APIs.
