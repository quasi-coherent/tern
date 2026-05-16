//! A migration for a database.
//!
//! # Description
//!
//! This module defines a single [`Migration`] in a set of migrations; it is
//! the atomic unit of `tern`.
//!
//! The [`Query`] of a migration, the real substance, is also defined here, and
//! ways to build one more conveniently are also provided.
//!
//! # Migration
//!
//! [`Migration`] is the type that represents a state change of the database
//! that is to be applied and recorded.  In `tern`, a `Migration` defines a
//! context that it needs in order to be applied.
//!
//! The migration and its context are defined by the user, either directly or
//! indirectly.  The helper [`ResolveQuery`] indirectly defines a `Migration`
//! by defining the query that should by applied at the time it's called.
//!
//! This is a more typical use: write a [`Query`], return it in a `ResolveQuery`
//! implementation, then use the derive macro for `Migration`, described in the
//! main documentation.
//!
//! # Query
//!
//! `Query` is the value that holds the SQL to be applied during a migration and
//! it can come in one of two flavors.  One, a single [`Statement`] is possibly
//! many individual queries that are ran as one statement, i.e., in a database
//! transaction.  Or it can be an ordered collection of statements, each of
//! which consists of one or more individual queries that run together in a
//! transaction.
//!
//! A `Query` with [`Statements`] fits the use case of a migration with SQL that
//! cannot, or otherwise should not, be ran in a database transaction.  For
//! instance, it is an error to run certain index builds in a transaction in
//! PostgreSQL.
//!
//! ## Source annotations
//!
//! A .sql file containing many individual queries will be interpreted by
//! database engines as one prepared statement and ran in a transaction.
//! Sometimes this is undesirable and sometimes it is not even allowed.  To
//! facilitate a migration that should not run in a transaction, `tern`
//! understands certain annotations found in the .sql file:
//!
//! * `tern:noTransaction` needs to be on the first line in a comment.  It is
//!   what activates the non-transactional interpretation by `tern`.
//! * `tern:begin` can be written in a query that has `tern:noTransaction` to
//!   signal the opening of a group of statements that should be ran together in
//!   a transaction even if the query as a whole should not.
//! * `tern:end` ends the group of statements.
//!
//! _Note_: Parsing SQL is hard...  If issues arise where a query is not being
//! split into statements correctly, first try to see if the syntax can be
//! adjusted to be easier to parse without changing the meaning.  For instance,
//! complicated use of commenting can be the cause of such issues.  If that
//! doesn't work, you can provide a hint to the dialect of SQL being used,
//! which may resolve the problem.  This is done with the `noTransaction`
//! annotation:
//!
//! ```sql
//! -- tern:noTransaction,postgres declares the file to contain postgres syntax.
//! -- Other values that are accepted are `mysql` and `sqlite`.  The default is
//! -- "postgres".
//! ```
//!
//! And if that doesn't work, either create multiple migrations or open an issue
//! if that is not possible.
//!
//! # Example
//!
//! This is an example of a SQL source file that we want to run as a collection
//! of `Statements`.
//!
//! ```sql
//! -- tern:noTransaction
//! -- The previous line means we will create groups of SQL statements.
//!
//! -- The following line opens a statement group:
//! -- tern:begin
//! CREATE TABLE blah.whatev_fast (LIKE blah.whatev INCLUDING CONSTRAINTS);
//!
//! -- This index build will happen in a transaction with the `CREATE TABLE...`
//! -- statement.
//! CREATE INDEX blah_whatev_fast_ca_bring_idx
//!   ON blah.whatev_fast USING bring (created_at)
//!   WITH (pages_per_range=64);
//!
//! -- Make it efficient:
//! SELECT bring_summarize_new_values('blah_whatev_fast_ca_bring_idx');
//!
//! -- Now we're going to close the group with:
//! -- tern:end
//!
//! -- Omitting the tags implies a group of one statement.
//! SELECT 'Done!';
//! ```
use futures_core::future::BoxFuture;
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;

use crate::context::MigrationContext;
use crate::error::TernResult;

pub mod iter;
pub use iter::{DownMigrationSet, UpMigrationSet};

mod split;

pub mod resolve;
pub use resolve::ResolveQuery;

pub mod query;
pub use query::Query;

pub mod types;
pub use types::{Applied, MigrationId};

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

    /// Produce the query defining the migration from its context.
    fn query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>>;
}

/// `PostApply` is an action to take with this migration after it successfully
/// applied.
pub trait PostApply: Migration {
    /// Action with the `Applied` result.
    fn post_apply<'a>(
        &'a self,
        ctx: &'a mut <Self as Migration>::Ctx,
        applied: &'a Applied,
    ) -> BoxFuture<'a, TernResult<()>>;
}

/// A `StaticMigration` defines its query irrespective of a context.
///
/// A static .sql file, for instance, can be a `StaticMigration`.  This value
/// implements `Migration` for any context.
pub struct StaticMigration<Ctx> {
    id: MigrationId,
    query: Query,
    _c: PhantomData<Ctx>,
}

impl<Ctx: MigrationContext> StaticMigration<Ctx> {
    /// Create a new `StaticMigration` over `Ctx` from ID and query.
    pub fn new(id: MigrationId, query: Query) -> StaticMigration<Ctx> {
        StaticMigration { id, query, _c: PhantomData }
    }
}

impl<Ctx: MigrationContext> Migration for StaticMigration<Ctx> {
    type Ctx = Ctx;

    fn migration_id(&self) -> &MigrationId {
        &self.id
    }

    fn query<'a>(
        &'a self,
        _: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        Box::pin(async { Ok(self.query.clone()) })
    }
}

impl<Ctx> Debug for StaticMigration<Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticMigration")
            .field("id", &self.id)
            .field("query", &self.query)
            .finish()
    }
}

impl<Ctx> Clone for StaticMigration<Ctx> {
    fn clone(&self) -> Self {
        Self { id: self.id.clone(), query: self.query.clone(), _c: PhantomData }
    }
}

/// `UpMigration` is a migration to apply in order to increment the version.
///
/// An `UpMigration` can be created from any migration.  The distinct property
/// of this value is that it defines a [`PostApply`] action that updates the
/// associated migration history table by inserting a row for it.
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

    fn migration_id(&self) -> &MigrationId {
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
            ctx.insert_applied(applied).await?;
            Ok(())
        })
    }
}

impl<Ctx> Debug for UpMigration<Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UpMigration").field(&"dyn Migration<Ctx = Ctx>").finish()
    }
}

/// `DownMigration` is a migration to apply in order to decrement the version.
///
/// A `DownMigration` can be created from any migration.  The distinct property
/// of this value is that it defines a [`PostApply`] action that removes the
/// row in the migration history table for the version it reverted.
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

    fn migration_id(&self) -> &MigrationId {
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
            ctx.delete_applied(version).await?;
            Ok(())
        })
    }
}

impl<Ctx> Debug for DownMigration<Ctx> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DownMigration")
            .field(&"dyn Migration<Ctx = Ctx>")
            .finish()
    }
}
