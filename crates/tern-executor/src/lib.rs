//! # tern-executor
//!
//! This crate builds compatibility between the `tern` API and external database
//! client crates.
#![cfg_attr(docsrs, feature(doc_cfg))]
use futures_core::future::Future;
use std::ffi::OsStr;
use std::fmt::{self, Debug, Formatter};
use tern_core::error::{TernError, TernResult};

mod backend;

#[cfg(any(
    feature = "sqlx_mysql",
    feature = "sqlx_postgres",
    feature = "sqlx_sqlite"
))]
pub mod sqlx_executor;

/// Method to create and connect to some `MigrationExecutor`.
pub trait ExecutorOptions<Exec> {
    /// Try to create `Exec`, returning the result.
    fn connect(self) -> impl Future<Output = TernResult<Exec>> + Send;
}

/// A DB url used for one implementation of `ExecutorOptions`.
#[derive(Clone)]
pub struct ConnStr(#[allow(dead_code)] String);

impl ConnStr {
    /// New from arg.
    pub fn new<T: Into<String>>(db_url: T) -> Self {
        Self(db_url.into())
    }

    /// New from the environment.
    pub fn from_env<K: AsRef<OsStr>>(k: K) -> TernResult<Self> {
        std::env::var(k.as_ref()).map(Self).map_err(TernError::exec_err)
    }
}

impl Debug for ConnStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "REDACTED")
    }
}

impl From<String> for ConnStr {
    fn from(value: String) -> Self {
        Self(value)
    }
}
