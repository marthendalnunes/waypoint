//! REST API service implementation

mod base;
mod error;
mod handlers;
mod openapi;
mod state;

pub use base::RestService;
pub use error::RestError;
pub use state::{McpResourceReader, ResourceReader, RestState};
