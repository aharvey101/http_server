use std::io;

// Custom error types for better error handling
#[derive(Debug)]
pub enum ServerError {
    #[allow(dead_code)] // Used for IO error conversion
    IoError(io::Error),
    TimeoutError,
    #[allow(dead_code)] // Used for connection errors
    ConnectionError(String),
}

impl From<io::Error> for ServerError {
    fn from(error: io::Error) -> Self {
        ServerError::IoError(error)
    }
}
