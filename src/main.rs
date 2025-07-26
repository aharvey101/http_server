mod lib;
#[cfg(test)]
mod tests;

use lib::{HttpServer, ServerConfig};
use std::env;

fn main() {
    // Load configuration from file or use defaults
    let config_path = env::args().nth(1).unwrap_or_else(|| "server.toml".to_string());
    let config = ServerConfig::load_from_file_or_default(&config_path);
    
    // Create server from configuration
    let server = match HttpServer::from_config(config.clone()) {
        Ok(server) => server,
        Err(e) => {
            eprintln!("Failed to create server: {:?}", e);
            return;
        }
    };
    
    println!("🚀 HTTP Server with Configuration System:");
    println!("   📄 Config file: {}", config_path);
    println!("   🌐 Address: {}:{}", config.server.host, config.server.port);
    println!("   🧵 Worker threads: {}", config.threading.worker_threads);
    println!("   🔗 Max connections: {}", config.threading.max_concurrent_connections);
    println!("   💾 Connection pool: {} idle connections", config.connection.max_idle_connections);
    println!("   📁 Static files: {} ({})", 
        if config.static_files.enabled { "enabled" } else { "disabled" },
        config.static_files.directory
    );
    println!("   🔐 Authentication: {}", 
        if config.authentication.enabled { "enabled" } else { "disabled" }
    );
    println!("   📝 Logging: {} (level: {})", 
        if config.logging.enabled { "enabled" } else { "disabled" },
        config.logging.level
    );
    println!("");
    println!("📋 Available endpoints:");
    println!("   GET  /               - Home page");
    println!("   GET  /hello?name=X   - Greeting with query params");
    println!("   GET  /api/status     - JSON status endpoint");
    println!("   GET  /api/stats      - Performance statistics");
    println!("   POST /api/echo       - Echo request data");
    if config.authentication.enabled {
        println!("   GET  /admin          - Protected admin panel");
    }
    println!("   GET  /chunked        - Chunked encoding demo");
    if config.static_files.enabled {
        println!("   GET  /static/        - Static file directory");
    }
    println!("");
    println!("🔧 Test commands:");
    println!("   curl http://{}:{}/", config.server.host, config.server.port);
    println!("   curl http://{}:{}/api/stats", config.server.host, config.server.port);
    if config.authentication.enabled {
        if let Some((username, password)) = config.authentication.users.iter().next() {
            println!("   curl -u {}:{} http://{}:{}/admin", username, password, config.server.host, config.server.port);
        }
    }
    println!("   curl http://{}:{}/chunked", config.server.host, config.server.port);
    println!("");
    println!("💡 Usage: {} [config_file.toml]", env::args().next().unwrap_or_else(|| "server".to_string()));
    println!("");
    
    if let Err(e) = server.start() {
        eprintln!("Server error: {:?}", e);
    }
}
