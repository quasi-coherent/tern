//! This module contains assorted third-party database connection types that
//! implement [`Executor`](crate::context::Executor).
#[cfg(feature = "sqlx")]
pub mod sqlx_backend;
