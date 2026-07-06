//! # tern-core
//!
//! This crate contains the core API for `tern`.  It is not meant to be used
//! directly.
#![warn(missing_docs)]
pub mod context;
pub mod error;
mod internal;
pub mod migration;
pub mod ops;
pub mod query;
