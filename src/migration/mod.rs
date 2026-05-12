//! The `Migration` interface.
//!
//!
use futures_core::future::BoxFuture;
use std::marker::PhantomData;
use std::sync::Arc;
use tern_core::context::MigrationContext;
use tern_core::error::TernResult;
use tern_core::executor::Executor;
use tern_core::query::Query;

pub use tern_core::migration::{Applied, Migration, MigrationId};

pub mod migration_set;
pub use migration_set::MigrationIteratorExt;
pub use migration_set::{DownMigrationSet, UpMigrationSet};

/// Helper for implementing `Migration` for dynamic queries.
///
/// Defining this is a requirement of using the `Migration` derive macro on a
/// Rust migration.
///
/// `ResolveQuery` is the mechanism for defining a migration to be materialized
/// at the time of being applied.
pub trait ResolveQuery: Send + Sync {
    /// The context required to resolve the migration query.
    type Ctx: MigrationContext;

    /// How to initialize this value.
    fn init(
        ctx: &mut Self::Ctx,
    ) -> impl Future<Output = TernResult<Self>> + Send
    where
        Self: Sized;

    /// Resolve the query in context.
    fn resolve(
        &self,
        ctx: &mut Self::Ctx,
    ) -> impl Future<Output = TernResult<Query>> + Send;
}

/// `PostApply` is an action to take with a migration after it has been applied.
pub trait PostApply: Migration {
    /// Action to take with the `Applied` result.
    fn post_apply<'a>(
        &'a self,
        ctx: &'a mut <Self as Migration>::Ctx,
        applied: &'a Applied,
    ) -> BoxFuture<'a, TernResult<()>>;
}

/// `UpMigration` is a migration to apply in order to increment the version.
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

    /// Create a new up migration from ID that uses `ResolveQuery` for the
    /// implementation of `Migration`.
    pub fn from_resolve_query<R>(id: MigrationId) -> Self
    where
        R: ResolveQuery<Ctx = Ctx> + 'static,
    {
        let resolved = Resolved::<R>::new(id);
        Self::new(resolved)
    }
}

impl<Ctx: MigrationContext> Migration for UpMigration<Ctx> {
    type Ctx = Ctx;

    fn migration_id(&self) -> MigrationId {
        self.0.migration_id()
    }

    fn query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        self.0.query(ctx)
    }
}

impl<Ctx: MigrationContext> PostApply for UpMigration<Ctx> {
    fn post_apply<'a>(
        &'a self,
        ctx: &'a mut <Self as Migration>::Ctx,
        applied: &'a Applied,
    ) -> BoxFuture<'a, TernResult<()>> {
        Box::pin(async move {
            log::debug!(version = applied.version(); "insert applied");
            let history = ctx.history_table();
            ctx.executor_mut().insert_applied(history, applied).await?;
            Ok(())
        })
    }
}

/// `DownMigration` is a migration to apply in order to decrement the version.
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

    /// Create a new down migration from ID that uses `ResolveQuery` for the
    /// implementation of `Migration`.
    pub fn from_resolve_query<R>(id: MigrationId) -> Self
    where
        R: ResolveQuery<Ctx = Ctx> + 'static,
    {
        let resolved = Resolved::<R>::new(id);
        Self::new(resolved)
    }
}

impl<Ctx: MigrationContext> Migration for DownMigration<Ctx> {
    type Ctx = Ctx;

    fn migration_id(&self) -> MigrationId {
        self.0.migration_id()
    }

    fn query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        self.0.query(ctx)
    }
}

impl<Ctx: MigrationContext> PostApply for DownMigration<Ctx> {
    fn post_apply<'a>(
        &'a self,
        ctx: &'a mut <Self as Migration>::Ctx,
        applied: &'a Applied,
    ) -> BoxFuture<'a, TernResult<()>> {
        Box::pin(async move {
            let version = applied.version();
            let descr = applied.description_ref();
            log::debug!(version, descr:%; "delete applied");
            let history = ctx.history_table();
            ctx.executor_mut().delete_applied(history, version).await?;
            Ok(())
        })
    }
}

// Helper to implement `Migration` for the symbol `R: ResolveQuery`.
struct Resolved<R> {
    id: MigrationId,
    _r: PhantomData<R>,
}

impl<R> Resolved<R> {
    fn new(id: MigrationId) -> Self {
        Self { id, _r: PhantomData }
    }
}

impl<R: ResolveQuery> Migration for Resolved<R> {
    type Ctx = R::Ctx;

    fn migration_id(&self) -> MigrationId {
        self.id.clone()
    }

    fn query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        Box::pin(async move {
            let resolver = R::init(ctx).await?;
            resolver.resolve(ctx).await
        })
    }
}

/// Some re-exports.
///
/// This module re-exports external symbols in the public API and some trait
/// methods without the trait.  Mostly exists for derive macros to have a path
/// to types appearing in traits they implement.
///
/// It is uncommon to use this unless you are doing a lot by hand.
pub mod types {
    pub use super::MigrationIteratorExt as _;
    pub use super::PostApply as _;
    pub(crate) use crate::report::TryReport as _;
    pub use futures_core::future::BoxFuture;
}
