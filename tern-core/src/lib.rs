//! The core of the [tern][tern-docs] migration library.
//!
//! This crate has types and traits for building migrations and running them
//! with a third-party database crate of choice.
//!
//! [tern-docs]: https://docs.rs/crate/tern/latest
#![cfg_attr(docsrs, feature(doc_cfg))]
pub mod backend;
pub mod context;
pub mod error;
pub mod source;
