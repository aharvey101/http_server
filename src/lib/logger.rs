use std::time::{SystemTime, UNIX_EPOCH};

// Logger for comprehensive logging
pub struct Logger {
}

impl Logger {
    pub fn new() -> Self {
        Logger {
        }
    }

    pub fn log_info(&self, message: &str) {
        let timestamp = self.get_timestamp();
        println!("[{}] INFO: {}", timestamp, message);
    }

    pub fn log_error(&self, message: &str) {
        let timestamp = self.get_timestamp();
        eprintln!("[{}] ERROR: {}", timestamp, message);
    }

    pub fn log_warning(&self, message: &str) {
        let timestamp = self.get_timestamp();
        println!("[{}] WARNING: {}", timestamp, message);
    }

    pub fn log_request(&self, method: &str, path: &str, status: u16, client_addr: &str) {
        let timestamp = self.get_timestamp();
        println!("[{}] {} {} - {} {}", timestamp, client_addr, method, path, status);
    }

    fn get_timestamp(&self) -> String {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs();
                let hours = (secs / 3600) % 24;
                let minutes = (secs / 60) % 60;
                let seconds = secs % 60;
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            }
            Err(_) => "00:00:00".to_string(),
        }
    }
}
