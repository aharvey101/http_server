pub mod error;
pub mod logger;
pub mod request;
pub mod response;
pub mod route;
pub mod router;
pub mod thread_pool;
pub mod connection_pool;
pub mod buffered_stream;
pub mod server;
pub mod auth;
pub mod config;

// Re-export commonly used types
pub use error::ServerError;
pub use logger::Logger;
pub use request::HttpRequest;
pub use response::HttpResponse;
pub use route::Route;
pub use router::Router;
pub use thread_pool::ThreadPool;
pub use connection_pool::ConnectionPool;
pub use buffered_stream::BufferedStream;
pub use server::HttpServer;
pub use auth::base64_decode;
pub use config::ServerConfig;
