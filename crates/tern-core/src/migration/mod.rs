//! Migrations for a database.
use futures_core::future::BoxFuture;
use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use crate::context::MigrationContext;
use crate::error::TernResult;
use crate::query::Query;

pub mod future;
use future::PendingMigration;
pub use future::ResolveMigration;

pub mod iter;
pub use iter::{DownMigrationSet, UpMigrationSet};

mod types;
pub use types::{HistoryTable, MigrationData, MigrationId};

/// An individual migration.
///
/// A `Migration` defines what context, if any, it needs in order to be applied
/// and it defines the query that should be sent when going to apply it.
pub trait Migration: Send + Sync {
    /// The context needed to create and apply this migration.
    type Ctx: MigrationContext;

    /// A migration has a version and name. This method returns a reference to
    /// these values collected in a `MigrationId`.
    fn migration_id(&self) -> &MigrationId;

    /// Produce the query defining this migration, which may or may not make use
    /// of the context that is supplied to the method.
    fn query(&self) -> TernResult<Query>;

    /// Resolve the query in an asynchronous context.
    ///
    /// By default this is just [`query`](Migration::query) put in a future that
    /// returns immediately, but it should be overridden when relied on for the
    /// migration context and/or the asynchronous setting.
    ///
    /// A dynamic migration uses this to supply operations that ask for the
    /// query but not for it to be applied, e.g., in a "dryrun" scenario.
    fn resolve_query<'a>(
        &'a self,
        _: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        Box::pin(async { self.query() })
    }

    /// Return the version alone.
    fn version(&self) -> i64 {
        self.migration_id().version()
    }
}

impl<M, D> Migration for D
where
    D: Deref<Target = M> + Send + Sync,
    for<'d> M: Migration + 'd,
{
    type Ctx = M::Ctx;

    fn migration_id(&self) -> &MigrationId {
        self.deref().migration_id()
    }

    fn query(&self) -> TernResult<Query> {
        self.deref().query()
    }

    fn resolve_query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        self.deref().resolve_query(ctx)
    }
}

/// `UpMigration` is a migration to apply in order to increment the version.
///
/// An `UpMigration` can be created from any migration, but when called in an
/// operation will be coupled with the operation to _insert_ the row for it
/// into the history table.
#[derive(Clone)]
pub struct UpMigration<Ctx>(Arc<dyn Migration<Ctx = Ctx>>);

impl<Ctx: MigrationContext> UpMigration<Ctx> {
    /// Create a new up migration from `M`.
    pub fn new<M>(migration: M) -> Self
    where
        M: Migration<Ctx = Ctx> + 'static,
    {
        Self(Arc::new(migration))
    }

    /// Create a new pending up migration from `R`.
    pub fn new_pending<R>(id: MigrationId) -> Self
    where
        R: ResolveMigration<Ctx = Ctx> + 'static,
    {
        let pending = PendingMigration::<R>::new(id);
        Self(Arc::new(pending))
    }
}

impl<Ctx: MigrationContext> Migration for UpMigration<Ctx> {
    type Ctx = Ctx;

    fn migration_id(&self) -> &MigrationId {
        self.0.migration_id()
    }

    fn query(&self) -> TernResult<Query> {
        self.0.query()
    }

    fn resolve_query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        self.0.resolve_query(ctx)
    }
}

impl<Ctx> Debug for UpMigration<Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UpMigration").field(&"dyn Migration<Ctx = Ctx>").finish()
    }
}

/// `DownMigration` is a migration to apply in order to decrement the version.
///
/// An `UpMigration` can be created from any migration, but when called in an
/// operation will be coupled with the operation to _delete_ the row for it from
/// the history table.
#[derive(Clone)]
pub struct DownMigration<Ctx>(Arc<dyn Migration<Ctx = Ctx>>);

impl<Ctx: MigrationContext> DownMigration<Ctx> {
    /// Create a new down migration from `M`.
    pub fn new<M>(migration: M) -> Self
    where
        M: Migration<Ctx = Ctx> + 'static,
    {
        Self(Arc::new(migration))
    }

    /// Create a new pending down migration from `R`.
    pub fn new_pending<R>(id: MigrationId) -> Self
    where
        R: ResolveMigration<Ctx = Ctx> + 'static,
    {
        let pending = PendingMigration::<R>::new(id);
        Self(Arc::new(pending))
    }
}

impl<Ctx: MigrationContext> Migration for DownMigration<Ctx> {
    type Ctx = Ctx;

    fn migration_id(&self) -> &MigrationId {
        self.0.migration_id()
    }

    fn query(&self) -> TernResult<Query> {
        self.0.query()
    }

    fn resolve_query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        self.0.resolve_query(ctx)
    }
}

impl<Ctx> Debug for DownMigration<Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DownMigration")
            .field(&"dyn Migration<Ctx = Ctx>")
            .finish()
    }
}
