//! # tern-executor
//!
//! This crate contains implementations of [`MigrationExecutor`] for foreign
//! types.
//!
//! [`MigrationExecutor`]: tern_core::executor::MigrationExecutor
#[cfg(feature = "sqlx")]
pub mod sqlx_executor;
