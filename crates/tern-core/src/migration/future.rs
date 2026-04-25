use futures_core::future::{BoxFuture, Future};
use std::marker::PhantomData;

use crate::context::MigrationContext;
use crate::error::TernResult;
use crate::migration::{Migration, MigrationId, Query};

/// Migration that is not statically defined.
///
/// `ResolveMigration` defines an interface for injecting a custom user context
/// via `MigrationContext` that creates the migration at the time of being
/// applied.
pub trait ResolveMigration: Send + Sync {
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

/// A migration that does not exist yet.
///
/// This value implements [`Migration`] by returning a placeholder query for the
/// synchronous method and it applies by initializing `R`, using it to resolve a
/// query, then calling on the context to send it.
pub struct PendingMigration<R> {
    id: MigrationId,
    _r: PhantomData<R>,
}

impl<R: ResolveMigration> PendingMigration<R> {
    /// New `PendingMigration`.
    pub fn new(id: MigrationId) -> Self {
        Self { id, _r: PhantomData }
    }
}

impl<R: ResolveMigration> Migration for PendingMigration<R> {
    type Ctx = R::Ctx;

    fn migration_id(&self) -> &MigrationId {
        &self.id
    }

    fn query(&self) -> TernResult<Query> {
        Ok(Query::pending())
    }

    fn resolve_query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        Box::pin(async move {
            let resolver = R::init(ctx).await?;
            resolver.resolve(ctx).await
        })
    }
}
