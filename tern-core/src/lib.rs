//! The core of the [`tern`][tern-docs] migration library.
//!
//! This crate has types and traits for building migrations and running them
//! with a third-party database crate of choice.
//!
//! [tern-docs]: https://docs.rs/crate/tern/latest
//! [Executor]: self::executor::Executor
//! [executor]: self::executor
#![cfg_attr(docsrs, feature(doc_cfg))]
pub mod error;
pub mod executor;
pub mod migration;
pub mod runner;

pub mod future {
    pub use futures_core::future::{BoxFuture, Future};
}
