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
#[cfg(feature = "cli")]
pub use app::cli::TernCli;
pub use app::{AppOptions, Tern, TernApp};

pub mod exec;
pub mod migration;
pub mod ops;

pub use tern_core::context::MigrationContext;
pub use tern_core::migration::Migration;

#[cfg(feature = "testing")]
#[cfg_attr(docsrs, doc(cfg(feature = "testing")))]
pub mod testing;

#[doc(inline)]
pub use tern_core::error::{self, TernError, TernResult};

#[doc(hidden)]
extern crate tern_derive;

pub use tern_derive::{Migration, TernApp};

#[cfg(feature = "testing")]
#[cfg_attr(docsrs, doc(cfg(feature = "testing")))]
pub use tern_derive::TernTest;

// Symbols needed by proc macros.
#[doc(hidden)]
pub mod private {
    pub use futures_core::future::BoxFuture;
}
