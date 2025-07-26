use api::{hash_password, generate_salt};
use std::env;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        // Password provided as command line argument
        let password = &args[1];
        let salt = generate_salt();
        let hashed = hash_password(password, &salt);
        println!("Hashed password: {}", hashed);
    } else {
        // Interactive mode
        print!("Enter password to hash: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        let password = input.trim();
        
        let salt = generate_salt();
        let hashed = hash_password(password, &salt);
        println!("Hashed password: {}", hashed);
        println!("\nYou can use this hash in your configuration file.");
        println!("Example:");
        println!("  users.insert(\"username\".to_string(), \"{}\".to_string());", hashed);
    }
}
