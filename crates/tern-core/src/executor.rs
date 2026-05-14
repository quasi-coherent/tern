//! Database client interaction for migrations.
use futures_core::Future;
use std::fmt::{self, Display, Formatter};

use crate::error::TernResult;
use crate::migration::Applied;
use crate::query::Query;

/// `Executor` is the database client interface for migration operations.
pub trait Executor: Send + Sync + 'static {
    /// Apply the given migration query.
    fn apply(
        &mut self,
        query: &Query,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Check that the history table exists.
    ///
    /// This is called before every migration run.
    fn check_history(
        &mut self,
        history: HistoryTable,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// A `CREATE TABLE IF NOT EXISTS` query for the history table.
    fn create_history_if_not_exists(
        &mut self,
        history: HistoryTable,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// A `DROP TABLE` query for the history table.
    fn drop_history(
        &mut self,
        history: HistoryTable,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Return all rows of the history table.
    fn get_all_applied(
        &mut self,
        history: HistoryTable,
    ) -> impl Future<Output = TernResult<Vec<Applied>>> + Send;

    /// Insert a newly applied migration into the history table.
    fn insert_applied(
        &mut self,
        history: HistoryTable,
        applied: &Applied,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Delete the applied migration from the history table.
    fn delete_applied(
        &mut self,
        history: HistoryTable,
        version: i64,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Insert or update an applied migration in the history table.
    fn upsert_applied(
        &mut self,
        history: HistoryTable,
        applied: &Applied,
    ) -> impl Future<Output = TernResult<()>> + Send;
}

/// The database table to record migration history in.
///
/// This defaults to "_tern_migrations" so will be created in the default
/// schema according to search path.
#[derive(Debug, Clone, Copy)]
pub struct HistoryTable {
    namespace: Option<&'static str>,
    name: &'static str,
}

impl HistoryTable {
    /// New in the default namespace.
    pub fn new(name: &'static str) -> Self {
        Self { namespace: None, name }
    }

    /// Specify a non-default namespace for the history table.
    pub fn in_namespace(self, schema: &'static str) -> Self {
        Self { namespace: Some(schema), name: self.name }
    }

    /// Returns the optional namespace or schema of the history table.
    pub fn namespace(&self) -> Option<&'static str> {
        self.namespace
    }

    /// Returns the name of the history table.
    pub fn tablename(&self) -> &'static str {
        self.name
    }

    /// Returns the history table in the format `(schema.)table`.
    pub fn full_name(&self) -> String {
        if let Some(schema) = self.namespace {
            return format!("{schema}.{}", self.name);
        }
        self.name.to_string()
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self { namespace: None, name: "_tern_migrations" }
    }
}

impl Display for HistoryTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.full_name())
    }
}
