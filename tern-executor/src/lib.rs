//! # tern-executor
//!
//! This crate builds compatibility between the `tern` API and external database
//! client crates.
#![cfg_attr(docsrs, feature(doc_cfg))]
mod backend;

#[cfg(feature = "_sqlx")]
pub mod sqlx_executor;
