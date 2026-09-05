//! # tern-test
//!
//! Runtime support for testing `tern` migration sets.  This crate is not
//! intended to be used directly.
use tern_core::context::MigrationExecutor;
use tern_core::error::TernResult;

mod conn;
pub use conn::TestConn;

pub mod property;
pub mod report;

#[cfg(any(
    feature = "sqlx_mysql",
    feature = "sqlx_postgres",
    feature = "sqlx_sqlite"
))]
pub mod sqlx_executor;

/// Build the single-threaded tokio runtime to drive one test suite.
pub fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed building the tern test runtime")
}

/// An executor that can provision a test database.
pub trait TestDatabase: MigrationExecutor {
    /// Provision a test database.
    fn create_database(
        &mut self,
        dbname: &str,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Drop the provisioned test database.
    fn drop_database(
        &mut self,
        dbname: &str,
    ) -> impl Future<Output = TernResult<()>> + Send;
}
