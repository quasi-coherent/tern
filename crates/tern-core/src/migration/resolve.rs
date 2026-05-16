//! Helper traits for implementing `Migration`.
//!
//! These mostly exist as implementation detail.  To use `tern` derive macros, a
//! type with `#[derive(Migration)]` is expected to implement [`ResolveQuery`].
//! Under this assumption the rest is filled in.
use futures_core::future::BoxFuture;

use crate::context::MigrationContext;
use crate::error::TernResult;
use crate::migration::types::HasMigrationId;
use crate::migration::{Migration, MigrationId, Query};

/// Helper for implementing `Migration` for dynamic queries.
///
/// `ResolveQuery` is the mechanism for defining a migration to be materialized
/// at the time of being applied.  It is a necessary component to be able to
/// derive `Migration`.
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

impl<M: ResolveQuery + HasMigrationId> Migration for M {
    type Ctx = <M as ResolveQuery>::Ctx;

    fn migration_id(&self) -> &MigrationId {
        self.id_ref()
    }

    fn query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        Box::pin(async move { self.resolve(ctx).await })
    }
}
