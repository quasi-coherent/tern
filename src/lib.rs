//! # tern
//!
//! See the [README][readme] to get started or browse the [examples][egs].
//! Have a question or problem?  Open a [discussion][disc] or [issue][iss]!
//!
//! [readme]: https://github.com/quasi-coherent/tern/blob/master/README.md
//! [disc]: https://github.com/quasi-coherent/tern/discussions
//! [iss]: https://github.com/quasi-coherent/tern/issues
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
mod app;
pub use app::{ContextOptions, Tern, TernApp};

#[cfg(feature = "cli")]
mod cli;

pub mod executor;
pub mod ops;

pub mod migration;
pub use migration::{Migration, Query, ResolveMigration, query};

pub use tern_core::context::MigrationContext;
#[doc(inline)]
pub use tern_core::error::{self, TernError, TernResult};

// Symbols needed by proc macros.
#[doc(hidden)]
pub mod private {
    pub use futures_core::future::BoxFuture;
}

#[doc(hidden)]
extern crate tern_derive;

pub use tern_derive::{Migration, TernApp};
