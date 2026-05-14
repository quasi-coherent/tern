//! # tern
//!
//! A bilingual Rust framework for managing migrations targeting a PostgreSQL,
//! MySQL, or SQLite backend.
//!
//! # Overview
#![cfg_attr(docsrs, feature(docs_cfg))]
#![warn(missing_docs)]
pub mod executor;
pub use executor::HistoryTable;

pub mod migrate;
pub use migrate::{Tern, TernMigrate};

pub mod migration;
pub use migration::{Migration, MigrationId, ResolveQuery};

pub mod operation;

pub mod report;
pub use report::Report;

pub use tern_core::context::MigrationContext;
#[doc(inline)]
pub use tern_core::error::{self, TernError, TernResult};
#[doc(inline)]
pub use tern_core::query::{self, Query};

#[doc(hidden)]
extern crate tern_derive;

pub use tern_derive::{Migration, TernMigrate};
