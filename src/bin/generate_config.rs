use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <config_file.toml>", args[0]);
        eprintln!("Generates a default configuration file for the HTTP server");
        process::exit(1);
    }
    
    let config_path = &args[1];
    
    // Create default config
    let config = api::ServerConfig::default();
    
    match config.save_to_file(config_path) {
        Ok(()) => {
            println!("✅ Default configuration written to: {}", config_path);
            println!("📝 Edit the file to customize your server settings");
        },
        Err(e) => {
            eprintln!("❌ Failed to write config file: {}", e);
            process::exit(1);
        }
    }
}
