use std::io;

// Custom error types for better error handling
#[derive(Debug)]
pub enum ServerError {
    IoError(io::Error),
    TimeoutError,
    ConnectionError(String),
}

impl From<io::Error> for ServerError {
    fn from(error: io::Error) -> Self {
        ServerError::IoError(error)
    }
}
