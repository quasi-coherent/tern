//! Context for a set of migrations.
use futures_core::future::{BoxFuture, Future};
use std::ffi::OsStr;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Deref;

use crate::error::{TernError, TernResult};
use crate::migration::MigrationData;
use crate::query::Query;

/// Main context provider for migrations.
pub trait MigrationContext: Send + Sync {
    /// The type of value used for low level database interaction.
    type Executor: MigrationExecutor;

    /// Get a mutable reference to this context's executor.
    fn executor_mut(&mut self) -> &mut Self::Executor;

    /// Return the location of the history table to record this context's
    /// results in.
    fn history_table(&self) -> HistoryRelid;
}

/// `MigrationExecutor` is the database client interface for migration
/// operations and interacting with the history table.
pub trait MigrationExecutor: Sized + Send + Sync + 'static {
    /// A `MigrationExecutor` is created from a connection string.
    fn connect(conn: &ConnStr)
    -> impl Future<Output = TernResult<Self>> + Send;

    /// Send the database query in a transaction.
    fn send_tx(
        &mut self,
        query: &str,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Send the database query _not_ in a transaction.
    fn send_notx(
        &mut self,
        query: &str,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// A query to check existence of the table.
    fn history_exists(
        &mut self,
        history: HistoryRelid,
    ) -> impl Future<Output = TernResult<bool>> + Send;

    /// Create the history table if it does not exist.
    fn create_if_not_exists(
        &mut self,
        history: HistoryRelid,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Drop the table if it exists.
    fn drop_if_exists(
        &mut self,
        history: HistoryRelid,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Return the applied migrations in the specified range of versions.
    ///
    /// A `None` value for `min_version` (resp. `max_version`) is interpreted as
    /// "the beginning" (resp. "the end").
    fn select_where_version_between(
        &mut self,
        history: HistoryRelid,
        min_version: Option<i64>,
        max_version: Option<i64>,
    ) -> impl Future<Output = TernResult<Vec<MigrationData>>> + Send;

    /// Insert a row.
    fn insert_into(
        &mut self,
        history: HistoryRelid,
        data: &MigrationData,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Delete a row.
    fn delete_from(
        &mut self,
        history: HistoryRelid,
        version: i64,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Insert or modify a row.
    fn insert_or_update(
        &mut self,
        history: HistoryRelid,
        data: &MigrationData,
    ) -> impl Future<Output = TernResult<()>> + Send;

    /// Send the database query.
    fn send<'a>(
        &'a mut self,
        query: &'a Query,
    ) -> BoxFuture<'a, TernResult<()>> {
        Box::pin(async move {
            if query.no_tx {
                let tot = query.size();

                for stat in query.inner.iter() {
                    let sql = stat.raw();
                    let idx = stat.idx;
                    log::trace!(
                        sql:%,
                        idx:%,
                        tot:%,
                        transaction = false;
                        "send statement",
                    );
                    self.send_notx(sql)
                        .await
                        .map_err(|e| TernError::stat(e, idx))?;
                }
            } else {
                let sql = query.into_raw();
                log::trace!(sql:%, transaction = true; "send statement");
                self.send_tx(&sql).await?;
            }

            Ok(())
        })
    }

    /// Get the current version, i.e., the latest applied migration.
    ///
    /// Returns `None` if there are no applied migrations.
    fn current_version(
        &mut self,
        history: HistoryRelid,
    ) -> BoxFuture<'_, TernResult<Option<MigrationData>>> {
        Box::pin(async move {
            let latest = self
                .select_where_version_between(history, None, None)
                .await?
                .into_iter()
                .fold(None::<MigrationData>, |acc, m| {
                    if acc.as_ref().is_none_or(|a| a.version() < m.version()) {
                        Some(m)
                    } else {
                        acc
                    }
                });

            Ok(latest)
        })
    }
}

/// A DB connection string.
#[derive(Clone)]
pub struct ConnStr(String);

impl ConnStr {
    /// New from stringlike.
    pub fn new<T: Into<String>>(db_url: T) -> Self {
        Self(db_url.into())
    }

    /// New from the environment.
    pub fn from_env<K: AsRef<OsStr>>(k: K) -> TernResult<Self> {
        std::env::var(k.as_ref()).map(Self).map_err(TernError::exec_err)
    }

    /// Try to parse into a [`url::Url`].
    pub fn try_into_url(&self) -> TernResult<url::Url> {
        url::Url::parse(&self.0).map_err(TernError::exec_err)
    }

    /// Return the DB URL scheme or an error if one could not be extracted.
    pub fn scheme(&self) -> Option<&str> {
        let mut sch = self.0.split(":");
        sch.next()
    }

    /// Consume this type and return connection string as a `String`.
    pub fn into_inner(self) -> String {
        self.0
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

impl Deref for ConnStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The relation where migration history is to be stored..
///
/// A [`MigrationContext`] supplies the `HistoryRelid` of the migration state
/// table to update with the outcome of operations.
#[derive(Debug, Clone, Copy)]
pub struct HistoryRelid {
    relschema: Option<&'static str>,
    relname: &'static str,
}

impl HistoryRelid {
    /// New table `relname` in the `relschema` schema.
    ///
    /// When omitted, the relation's schema is whatever the default is as
    /// determined by the connection (e.g., from the search path).
    pub const fn new(relname: &'static str) -> Self {
        Self { relschema: None, relname }
    }

    /// Set the relation namespace.
    pub const fn set_relschema(self, schema: &'static str) -> HistoryRelid {
        let HistoryRelid { relname, .. } = self;
        HistoryRelid { relschema: Some(schema), relname }
    }

    /// Return the schema where the relation is.
    pub const fn relschema(&self) -> Option<&'static str> {
        self.relschema
    }

    /// Return the relation's name.
    pub const fn relname(&self) -> &'static str {
        self.relname
    }

    /// Returns the relation in the format `<schema>.<name>`.
    pub fn scoped_table(&self) -> String {
        let name = self.relname();
        if let Some(schema) = self.relschema() {
            return format!("{schema}.{name}");
        }
        name.to_string()
    }
}

impl Default for HistoryRelid {
    fn default() -> Self {
        Self { relschema: None, relname: "_tern_migrations" }
    }
}

impl Display for HistoryRelid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.scoped_table())
    }
}
