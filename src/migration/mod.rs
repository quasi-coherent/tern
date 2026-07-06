//! Database migration sets.
//!
//! This module exports collections of `Migration`s in [`MigrationSet`] and
//! [`Invertible`], which are the objects that most operations are parametrized
//! over.
use futures_core::future::BoxFuture;
use tern_core::context::MigrationContext;
use tern_core::error::TernResult;
use tern_core::query::Query;

pub use tern_core::context::RelationId;
pub use tern_core::migration::{MigrationData, MigrationId};

mod boxed;
pub use boxed::{MigrationBox, MigrationBoxSet};

/// A helper trait for migration queries.
///
/// Provide by-hand where a type derives [`Migration`] to complete the
/// implementation.
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

    /// Future resolving to the down migration query if defined.
    ///
    /// The provided implementation returns `Ok(None)`.  Override to supply a
    /// down migration query partner.
    fn resolve_revert_query<'a>(
        &'a self,
        #[allow(unused_variables)] ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Option<Query>>> {
        Box::pin(core::future::ready(Ok(None)))
    }
}
