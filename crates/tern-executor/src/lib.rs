//! # tern-executor
//!
//! This crate builds compatibility between the `tern` API and external database
//! client crates.
#![cfg_attr(docsrs, feature(doc_cfg))]
mod impls;
mod query;

#[cfg(feature = "sqlx")]
#[doc(hidden)]
pub extern crate sqlx as sqlx_lib;

#[cfg(feature = "sqlx")]
#[doc(inline)]
pub use impls::sqlx::{SqlxError, any::SqlxExecutor};

#[cfg(feature = "sqlx_mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_mysql")))]
#[doc(inline)]
pub use impls::sqlx::mysql::{self as sqlx_mysql, SqlxMySqlExecutor};

#[cfg(feature = "sqlx_postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_postgres")))]
#[doc(inline)]
pub use impls::sqlx::postgres::{self as sqlx_postgres, SqlxPgExecutor};

#[cfg(feature = "sqlx_sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_sqlite")))]
#[doc(inline)]
pub use impls::sqlx::sqlite::{self as sqlx_sqlite, SqlxSqliteExecutor};
