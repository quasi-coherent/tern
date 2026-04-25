#![cfg_attr(docsrs, feature(docs_cfg))]
#![warn(missing_docs)]
//! # tern
//!
//! A database migration library and tool with embedded migrations supporting SQL or Rust sources.
pub use tern_core::context::MigrationContext;
#[doc(inline)]
pub use tern_core::error::{self, TernError, TernResult};
pub use tern_core::executor::{HistoryTable, MigrationExecutor};
#[doc(inline)]
pub use tern_core::migration::{self, AppliedMigration, Migration};
#[doc(inline)]
pub use tern_core::query::{self, Query, QueryBuilder};

/// Migration query execution.
///
/// This module
pub mod executor {
    #[cfg_attr(feature = "_sqlx")]
    pub use tern_executor::sqlx_executor as sqlx;
}
