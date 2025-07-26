use std::collections::HashMap;
use std::fs;
use std::path::Path;
use super::auth::{hash_password, generate_salt};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub server: ServerSettings,
    pub threading: ThreadingSettings,
    pub connection: ConnectionSettings,
    pub static_files: StaticFilesSettings,
    pub authentication: AuthenticationSettings,
    pub logging: LoggingSettings,
}

#[derive(Debug, Clone)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub read_timeout_seconds: u64,
    pub write_timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct ThreadingSettings {
    pub worker_threads: usize,
    pub max_concurrent_connections: usize,
}

#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    pub max_idle_connections: usize,
    pub idle_timeout_seconds: u64,
    pub keep_alive_timeout_seconds: u64,
    pub buffer_size: usize,
}

#[derive(Debug, Clone)]
pub struct StaticFilesSettings {
    pub enabled: bool,
    pub directory: String,
    pub index_file: String,
    pub directory_listing: bool,
}

#[derive(Debug, Clone)]
pub struct AuthenticationSettings {
    pub enabled: bool,
    pub users: HashMap<String, String>, // username -> password
    pub protected_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LoggingSettings {
    pub enabled: bool,
    pub level: String, // "info", "warning", "error"
    pub log_requests: bool,
    pub log_responses: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let mut auth_users = HashMap::new();
        
        // Create hashed passwords for default users
        // admin:password123 -> hashed
        let admin_salt = generate_salt();
        let admin_hash = hash_password("password123", &admin_salt);
        auth_users.insert("admin".to_string(), admin_hash);
        
        // user:secret -> hashed
        let user_salt = generate_salt();
        let user_hash = hash_password("secret", &user_salt);
        auth_users.insert("user".to_string(), user_hash);

        ServerConfig {
            server: ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 8080,
                read_timeout_seconds: 30,
                write_timeout_seconds: 30,
            },
            threading: ThreadingSettings {
                worker_threads: 4,
                max_concurrent_connections: 100,
            },
            connection: ConnectionSettings {
                max_idle_connections: 20,
                idle_timeout_seconds: 30,
                keep_alive_timeout_seconds: 60,
                buffer_size: 8192, // 8KB
            },
            static_files: StaticFilesSettings {
                enabled: true,
                directory: "static".to_string(),
                index_file: "index.html".to_string(),
                directory_listing: true,
            },
            authentication: AuthenticationSettings {
                enabled: true,
                users: auth_users,
                protected_paths: vec!["/admin".to_string()],
            },
            logging: LoggingSettings {
                enabled: true,
                level: "info".to_string(),
                log_requests: true,
                log_responses: false,
            },
        }
    }
}

impl ServerConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let config_content = fs::read_to_string(path)
            .map_err(|e| ConfigError::FileRead(e.to_string()))?;
        
        Self::parse_toml(&config_content)
    }

    pub fn load_from_file_or_default<P: AsRef<Path>>(path: P) -> Self {
        match Self::load_from_file(path) {
            Ok(config) => config,
            Err(_) => {
                eprintln!("Warning: Could not load config file, using defaults");
                Self::default()
            }
        }
    }

    #[allow(dead_code)] // Public API method for config saving
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let toml_content = self.to_toml();
        fs::write(path, toml_content)
            .map_err(|e| ConfigError::FileWrite(e.to_string()))?;
        Ok(())
    }

    fn parse_toml(content: &str) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        
        // Simple TOML parsing - in a real implementation you'd use a TOML library
        // For now, we'll implement basic parsing for key-value pairs
        let lines: Vec<&str> = content.lines().collect();
        let mut current_section = "";
        
        for line in lines {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if line.starts_with('[') && line.ends_with(']') {
                current_section = &line[1..line.len()-1];
                continue;
            }
            
            if let Some(equals_pos) = line.find('=') {
                let key = line[..equals_pos].trim();
                let value = line[equals_pos + 1..].trim().trim_matches('"');
                
                match current_section {
                    "server" => Self::parse_server_setting(&mut config.server, key, value)?,
                    "threading" => Self::parse_threading_setting(&mut config.threading, key, value)?,
                    "connection" => Self::parse_connection_setting(&mut config.connection, key, value)?,
                    "static_files" => Self::parse_static_files_setting(&mut config.static_files, key, value)?,
                    "authentication" => Self::parse_auth_setting(&mut config.authentication, key, value)?,
                    "logging" => Self::parse_logging_setting(&mut config.logging, key, value)?,
                    _ => {} // Ignore unknown sections
                }
            }
        }
        
        Ok(config)
    }

    fn parse_server_setting(settings: &mut ServerSettings, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "host" => settings.host = value.to_string(),
            "port" => settings.port = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            "read_timeout_seconds" => settings.read_timeout_seconds = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            "write_timeout_seconds" => settings.write_timeout_seconds = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn parse_threading_setting(settings: &mut ThreadingSettings, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "worker_threads" => settings.worker_threads = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            "max_concurrent_connections" => settings.max_concurrent_connections = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn parse_connection_setting(settings: &mut ConnectionSettings, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "max_idle_connections" => settings.max_idle_connections = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            "idle_timeout_seconds" => settings.idle_timeout_seconds = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            "keep_alive_timeout_seconds" => settings.keep_alive_timeout_seconds = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            "buffer_size" => settings.buffer_size = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn parse_static_files_setting(settings: &mut StaticFilesSettings, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "enabled" => settings.enabled = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            "directory" => settings.directory = value.to_string(),
            "index_file" => settings.index_file = value.to_string(),
            "directory_listing" => settings.directory_listing = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn parse_auth_setting(settings: &mut AuthenticationSettings, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "enabled" => settings.enabled = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            _ if key.starts_with("user_") => {
                let username = &key[5..]; // Remove "user_" prefix
                settings.users.insert(username.to_string(), value.to_string());
            },
            _ if key.starts_with("protected_path_") => {
                settings.protected_paths.push(value.to_string());
            },
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    fn parse_logging_setting(settings: &mut LoggingSettings, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            "enabled" => settings.enabled = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            "level" => settings.level = value.to_string(),
            "log_requests" => settings.log_requests = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            "log_responses" => settings.log_responses = value.parse().map_err(|_| ConfigError::InvalidValue(key.to_string()))?,
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    #[allow(dead_code)] // Used by save_to_file method
    fn to_toml(&self) -> String {
        let mut toml = String::new();
        
        toml.push_str("# HTTP Server Configuration\n\n");
        
        toml.push_str("[server]\n");
        toml.push_str(&format!("host = \"{}\"\n", self.server.host));
        toml.push_str(&format!("port = {}\n", self.server.port));
        toml.push_str(&format!("read_timeout_seconds = {}\n", self.server.read_timeout_seconds));
        toml.push_str(&format!("write_timeout_seconds = {}\n\n", self.server.write_timeout_seconds));
        
        toml.push_str("[threading]\n");
        toml.push_str(&format!("worker_threads = {}\n", self.threading.worker_threads));
        toml.push_str(&format!("max_concurrent_connections = {}\n\n", self.threading.max_concurrent_connections));
        
        toml.push_str("[connection]\n");
        toml.push_str(&format!("max_idle_connections = {}\n", self.connection.max_idle_connections));
        toml.push_str(&format!("idle_timeout_seconds = {}\n", self.connection.idle_timeout_seconds));
        toml.push_str(&format!("keep_alive_timeout_seconds = {}\n", self.connection.keep_alive_timeout_seconds));
        toml.push_str(&format!("buffer_size = {}\n\n", self.connection.buffer_size));
        
        toml.push_str("[static_files]\n");
        toml.push_str(&format!("enabled = {}\n", self.static_files.enabled));
        toml.push_str(&format!("directory = \"{}\"\n", self.static_files.directory));
        toml.push_str(&format!("index_file = \"{}\"\n", self.static_files.index_file));
        toml.push_str(&format!("directory_listing = {}\n\n", self.static_files.directory_listing));
        
        toml.push_str("[authentication]\n");
        toml.push_str(&format!("enabled = {}\n", self.authentication.enabled));
        for (username, password) in &self.authentication.users {
            toml.push_str(&format!("user_{} = \"{}\"\n", username, password));
        }
        for (i, path) in self.authentication.protected_paths.iter().enumerate() {
            toml.push_str(&format!("protected_path_{} = \"{}\"\n", i + 1, path));
        }
        toml.push_str("\n");
        
        toml.push_str("[logging]\n");
        toml.push_str(&format!("enabled = {}\n", self.logging.enabled));
        toml.push_str(&format!("level = \"{}\"\n", self.logging.level));
        toml.push_str(&format!("log_requests = {}\n", self.logging.log_requests));
        toml.push_str(&format!("log_responses = {}\n", self.logging.log_responses));
        
        toml
    }

    pub fn get_bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    FileRead(String),
    #[allow(dead_code)] // Used by save_to_file method
    FileWrite(String),
    InvalidValue(String),
    UnknownKey(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileRead(err) => write!(f, "Failed to read config file: {}", err),
            ConfigError::FileWrite(err) => write!(f, "Failed to write config file: {}", err),
            ConfigError::InvalidValue(key) => write!(f, "Invalid value for config key: {}", key),
            ConfigError::UnknownKey(key) => write!(f, "Unknown config key: {}", key),
        }
    }
}

impl std::error::Error for ConfigError {}
