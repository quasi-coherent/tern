//! # tern
//!
//! A bilingual Rust framework for managing migrations targeting a PostgreSQL,
//! MySQL, or SQLite backend.
//!
//! # Overview
#![cfg_attr(docsrs, feature(docs_cfg))]
#![warn(missing_docs)]
mod app;
pub use app::Tern;

pub mod executor;
pub mod ops;
pub mod report;
pub use report::Report;

pub use tern_core::context::MigrationContext;
pub use tern_core::error::{self, TernError, TernResult};
pub use tern_core::migrate::{self, TernMigrate, TernMigrateOp, TernOptions};
pub use tern_core::migration::{self, Migration, Query, ResolveQuery};

// Re-export external symbol in the path for proc macros.
#[doc(hidden)]
pub mod private {
    pub use futures_core::future::BoxFuture;
}

#[doc(hidden)]
extern crate tern_derive;

pub use tern_derive::{Migration, TernMigrate};
