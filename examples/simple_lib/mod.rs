//! # simple
//!
//! This simple example shows how a custom context can be used to inject logic
//! into a migration at runtime.
use tern::error::{ErrorKind, MigrationError};

mod migrations;
pub use migrations::SimpleExample;

/// A custom error type.
///
/// It implements [`MigrationError`], so `?`-s into `TernError`.
///
/// [`MigrationError`]: tern::error::MigrationError
#[derive(Debug, thiserror::Error)]
pub enum ExampleError {
    #[error("variable not found in environment: {0}")]
    Unset(std::env::VarError),
    #[error("error writing to query: {0}")]
    Query(std::fmt::Error),
}

impl MigrationError for ExampleError {
    fn message(&self) -> String {
        self.to_string()
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Custom
    }
}
