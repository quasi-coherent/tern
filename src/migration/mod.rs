//! Database migration sets.
//!
//! This module exports collections of `Migration`s in [`MigrationSet`] and
//! [`Invertible`], which are the objects that most operations are parametrized
//! over.
use futures_core::future::BoxFuture;
use tern_core::context::MigrationContext;
use tern_core::error::TernResult;

pub use tern_core::context::HistoryRelid;
pub use tern_core::migration::{MigrationData, MigrationId};
#[doc(inline)]
pub use tern_core::query::{self, Query, QueryBuilder};

mod boxed;
pub use boxed::{MigrationBox, MigrationBoxSet, migration_box_set};

/// A helper trait for migration queries.
///
/// Required of a type that derives `Migration`.
pub trait ResolveQuery {
    /// The context associated with the migration this query is for.
    type Ctx: MigrationContext;

    /// Initialize this value.
    fn init(ctx: &mut Self::Ctx) -> BoxFuture<'_, TernResult<Self>>
    where
        Self: Sized;

    /// Future resolving to the up migration query.
    fn resolve_query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>>;
}

/// A helper trait for migration queries.
///
/// Required of a type that derives `Migration` and is part of an up/down set of
/// migrations.
pub trait ResolveRevertQuery: ResolveQuery {
    /// Future resolving to the down migration query.
    ///
    /// This should revert the effect of [`ResolveQuery::resolve_query`].
    fn resolve_revert_query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>>;
}
