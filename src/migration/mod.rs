//! The `Migration` interface.
//!
//!
use futures_core::future::BoxFuture;
use std::sync::Arc;
use tern_core::context::MigrationContext;
use tern_core::error::TernResult;
use tern_core::executor::Executor;

pub use tern_core::migration::{Applied, Migration, MigrationId};

pub mod migration_set;
pub use migration_set::{DownMigrationSet, UpMigrationSet};

mod resolve;
pub use resolve::{ResolveQuery, Resolved};

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
}

impl<Ctx: MigrationContext> Migration for UpMigration<Ctx> {
    type Ctx = Ctx;

    fn migration_id(&self) -> MigrationId {
        self.0.migration_id()
    }

    fn apply<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Applied>> {
        self.0.apply(ctx)
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
}

impl<Ctx: MigrationContext> Migration for DownMigration<Ctx> {
    type Ctx = Ctx;

    fn migration_id(&self) -> MigrationId {
        self.0.migration_id()
    }

    fn apply<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Applied>> {
        self.0.apply(ctx)
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
