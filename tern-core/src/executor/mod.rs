//! This module contains assorted third-party database connection types that
//! implement [`Executor`](crate::migration::Executor).
#[cfg(feature = "sqlx")]
pub mod sqlx_backend;
