//! The migration abstraction.
use chrono::{DateTime, Utc};
use futures_core::future::BoxFuture;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Deref;
use std::time::Duration;

use crate::context::MigrationContext;
use crate::error::TernResult;
use crate::query::Query;

/// A migration is represented as a future that resolves to a `Query`.
pub trait Migration: Send + Sync {
    /// The context needed in order to provide the resolved query.
    type Ctx: MigrationContext;

    /// Return a borrow of the migration ID.
    fn migration_id(&self) -> &MigrationId;

    /// Produce the future to resolve the query.
    fn query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>>;

    /// Produce the future to resolve the query reverting [`Self::query`] if the
    /// migration set supports it.
    ///
    /// By default this immediately returns `Ok(None)`.  Override to provide the
    /// the inverse query.
    fn revert_query<'a>(
        &'a self,
        #[allow(unused_variables)] ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Option<Query>>> {
        Box::pin(core::future::ready(Ok(None)))
    }

    /// Get the version of the source migration.
    fn version(&self) -> i64 {
        self.migration_id().version()
    }

    /// Return the name of the source.
    fn description(&self) -> &str {
        self.migration_id().description()
    }

    /// Return the query statically if that is possible.
    fn show_query(&self) -> Query {
        Query::pending()
    }
}

impl<Ctx: MigrationContext, M, D> Migration for D
where
    D: Deref<Target = M> + Send + Sync,
    for<'a> M: Migration<Ctx = Ctx> + 'a,
{
    type Ctx = M::Ctx;

    fn migration_id(&self) -> &MigrationId {
        M::migration_id(self)
    }

    fn query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        M::query(self, ctx)
    }

    fn revert_query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Option<Query>>> {
        M::revert_query(self, ctx)
    }
}

/// Identifier for a migration in a migration set.
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct MigrationId {
    version: i64,
    description: String,
}

impl MigrationId {
    /// New `MigrationId` a version and name/description.
    pub fn new<T: Into<String>>(version: i64, description: T) -> Self {
        Self { version, description: description.into() }
    }

    /// Return the integer version.
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Returns the description.
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl Display for MigrationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}__{}", self.version(), self.description())
    }
}

/// Migration metadata.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct MigrationData {
    version: i64,
    description: String,
    content: String,
    duration_ms: i64,
    applied_at: DateTime<Utc>,
}

impl MigrationData {
    /// New `MigrationData`.
    pub fn new(version: i64, description: &str, query: &Query) -> Self {
        Self {
            version,
            description: description.to_string(),
            content: query.to_string(),
            duration_ms: -1,
            applied_at: Default::default(),
        }
    }

    /// Update to reflect that the migration finished.
    pub fn finished(&mut self, started_at: DateTime<Utc>) {
        let applied_at = Utc::now();
        let duration_ms = (applied_at - started_at).num_milliseconds();
        self.applied_at = applied_at;
        self.duration_ms = duration_ms;
    }

    /// Return the integer version of the migration.
    pub fn version(&self) -> i64 {
        self.version
    }

    /// Returns a reference to the migration description or name.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns a reference to the raw content of the original migration source.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the duration in milliseconds of the migration query run.
    ///
    /// Note that this will return nonsensical results if the query did not have
    /// [`finished`](MigrationData::finished) called on it.
    pub fn duration_millis(&self) -> i64 {
        self.duration_ms
    }

    /// Returns the UTC timestamp of when the migration was applied.
    ///
    /// Note that this will return nonsensical results if the query did not have
    /// [`finished`](MigrationData::finished) called on it.
    pub fn applied_at(&self) -> DateTime<Utc> {
        self.applied_at
    }
}

impl Display for MigrationData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut lines = self.content().lines().take(6).collect::<Vec<_>>();
        let truncated = lines.pop().is_some();
        let snip = lines.join("\n");
        let version = self.version;
        let description = self.description();
        let content = if truncated { format!("{snip}...") } else { snip };
        let duration = self
            .duration_ms
            .try_into()
            .map(Duration::from_millis)
            .unwrap_or_default()
            .as_secs_f64();
        let applied_at = self.applied_at();

        write!(
            f,
            r#"
{{
  "version": {version},
  "description": "{description}",
  "content": "{content}",
  "duration": "{duration}s",
  "applied_at": "{applied_at}",
}}
"#,
        )
    }
}

/// Iterators over migrations.
pub mod iter {
    use super::*;

    /// Extension trait for `Iterator`s.
    pub trait MigrationSetExt<Ctx: MigrationContext>:
        DoubleEndedIterator
    {
        /// Limit an iterator of migrations to a range of versions.
        fn range(self, minv: Option<i64>, maxv: Option<i64>) -> Range<Self>
        where
            Self: Sized,
            Self::Item: Migration<Ctx: MigrationContext>,
        {
            Range::new(self, minv, maxv)
        }
    }

    impl<Ctx, S> MigrationSetExt<Ctx> for S
    where
        Ctx: MigrationContext,
        S: DoubleEndedIterator<Item: Migration<Ctx = Ctx> + 'static>,
    {
    }

    /// Range of migration versions from a migration set.
    pub struct Range<I> {
        iter: I,
        minv: Option<i64>,
        maxv: Option<i64>,
    }

    impl<I> Range<I> {
        fn new(iter: I, minv: Option<i64>, maxv: Option<i64>) -> Self {
            Self { iter, minv, maxv }
        }
    }

    impl<Ctx, I> Iterator for Range<I>
    where
        Ctx: MigrationContext,
        I: Iterator<Item: Migration<Ctx = Ctx> + 'static>,
    {
        type Item = I::Item;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(migration) = self.iter.next()
                && let ver = migration.version()
                && self.minv.is_none_or(|n| ver >= n)
                && self.maxv.is_none_or(|n| ver <= n)
            {
                Some(migration)
            } else {
                None
            }
        }
    }

    impl<Ctx, I> DoubleEndedIterator for Range<I>
    where
        Ctx: MigrationContext,
        I: DoubleEndedIterator<Item: Migration<Ctx = Ctx> + 'static>,
    {
        fn next_back(&mut self) -> Option<Self::Item> {
            if let Some(migration) = self.iter.next_back()
                && let ver = migration.version()
                && self.minv.is_none_or(|n| ver >= n)
                && self.maxv.is_none_or(|n| ver <= n)
            {
                Some(migration)
            } else {
                None
            }
        }
    }
}
