use chrono::Utc;
use futures_core::future::{BoxFuture, Future};
use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;
use tern_core::context::MigrationContext;
use tern_core::error::TernResult;
use tern_core::executor::Executor;
use tern_core::migration::{Applied, Migration, MigrationId};
use tern_core::query::Query;

/// A helper trait to build `Migration` for dynamic queries.
///
/// This allows a user to define a migration such that it and its query are
/// created from the arbitrary context at the time of being applied.
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

/// `Resolved` is a concrete `ResolveQuery`.
pub struct Resolved<R> {
    id: MigrationId,
    _r: PhantomData<R>,
}

impl<R: ResolveQuery> Resolved<R> {
    /// New `Resolved` value that will construct the migration with given ID.
    pub fn new(id: MigrationId) -> Self {
        Resolved { id, _r: PhantomData }
    }
}

impl<R: ResolveQuery> Migration for Resolved<R> {
    type Ctx = R::Ctx;

    fn migration_id(&self) -> MigrationId {
        self.id.clone()
    }

    fn apply<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Applied>> {
        Box::pin(async move {
            let start = Utc::now();
            let id = &self.id;

            log::debug!(id:%; "initializing");
            let resolver = R::init(ctx).await?;

            log::debug!(id:%; "resolving migration");
            let query = resolver.resolve(ctx).await?;

            log::debug!(id:%, query:%; "applying migration");
            ctx.executor_mut().apply(&query).await?;

            log::debug!(id:%; "applied migration");
            let content = query.to_string();
            let applied = Applied::new(id, content, start);

            Ok(applied)
        })
    }
}

impl<R> Clone for Resolved<R> {
    fn clone(&self) -> Self {
        Resolved { id: self.id.clone(), _r: PhantomData }
    }
}

impl<R> Display for Resolved<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}
