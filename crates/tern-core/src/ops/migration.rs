//! Operations to apply migrations.
use chrono::Utc;
use std::marker::PhantomData;

use crate::context::{MigrationContext, MigrationExecutor};
use crate::error::{ResultWithId as _, TernError, TernResult};
use crate::migration::{DownMigration, Migration, MigrationData, UpMigration};
use crate::ops::MigrationOp;
use crate::ops::crud::{DeleteApplied, UpdateApplied};
use crate::query::Query;

/// Operation to return the query of a migration.
///
/// This only calls `query`, so returns a placeholder if the migration's query
/// is dynamically defined.
pub struct StaticQuery<'a, Ctx>(PhantomData<&'a mut Ctx>);

impl<'a, Ctx> StaticQuery<'a, Ctx> {
    /// New `StaticQuery` operation.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a, Ctx> Default for StaticQuery<'a, Ctx> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<M, Ctx> MigrationOp<&M> for StaticQuery<'_, Ctx>
where
    Ctx: MigrationContext,
    M: Migration<Ctx = Ctx>,
{
    type Output = TernResult<Query>;

    async fn exec(self, input: &M) -> Self::Output {
        let id = input.migration_id();
        let q = input.query().map_err_id(id)?;
        log::debug!(id:%, query:% = q; "static query");
        Ok(q)
    }
}

/// Operation to resolve the query of a migration.
pub struct RenderQuery<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx> RenderQuery<'a, Ctx> {
    /// New `RenderQuery` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<M, Ctx> MigrationOp<&M> for RenderQuery<'_, Ctx>
where
    Ctx: MigrationContext,
    M: Migration<Ctx = Ctx>,
{
    type Output = TernResult<Query>;

    async fn exec(self, input: &M) -> Self::Output {
        let id = input.migration_id();
        let q = input.resolve_query(self.0).await.map_err_id(id)?;
        log::debug!(id:%, query:% = q; "rendered query");
        Ok(q)
    }
}

/// An operation for applying one up migration.
///
/// Note that this also runs the operation `InsertMigrationData` on success, so
/// it is not necessary to run the insert when this completes.
#[derive(Debug)]
pub struct Up<'a, Ctx> {
    ctx: &'a mut Ctx,
    dryrun: bool,
    soft_apply: bool,
}

impl<'a, Ctx> Up<'a, Ctx> {
    /// New `Up` operation.
    pub fn new_apply(ctx: &'a mut Ctx) -> Self {
        Self { ctx, dryrun: false, soft_apply: false }
    }

    /// New `Up` inserting into history but skipping the migration query.
    pub fn new_soft_apply(ctx: &'a mut Ctx) -> Self {
        Self { ctx, dryrun: false, soft_apply: true }
    }

    /// Set this to be a dryrun.
    pub fn dryrun(self) -> Up<'a, Ctx> {
        Up { ctx: self.ctx, dryrun: true, soft_apply: self.soft_apply }
    }

    /// `true` if this is a dryrun.
    pub fn is_dryrun(&self) -> bool {
        self.dryrun
    }

    /// `true` if only the history table will update.
    pub fn is_soft_apply(&self) -> bool {
        self.soft_apply
    }
}

impl<Ctx: MigrationContext> MigrationOp<&UpMigration<Ctx>> for Up<'_, Ctx> {
    type Output = TernResult<MigrationData>;

    async fn exec(self, input: &UpMigration<Ctx>) -> Self::Output {
        let id = input.migration_id();
        let start = Utc::now();

        let applied = async {
            let q = RenderQuery::new(self.ctx).exec(input).await?;
            if self.is_dryrun() {
                return Ok::<_, TernError>(MigrationData::new(id, &q, start));
            }
            if !self.is_soft_apply() {
                self.ctx.executor_mut().send(&q).await?;
                log::debug!(id:%; "applied migration");
            }
            let applied = MigrationData::new(id, &q, start);

            // If not a "soft" apply, the record should not exist in history.
            // The `UpdateMigrationData` operation should do the same thing in
            // either case though.
            UpdateApplied::new(self.ctx).exec(&applied).await?;

            Ok(applied)
        }
        .await
        .map_err_id(id)?;

        Ok(applied)
    }
}

/// An operation for applying one down migration.
///
/// Note that this also runs the operation `DeleteMigrationData` on success, so
/// it is not necessary to follow up this operation with that one.
#[derive(Debug)]
pub struct Down<'a, Ctx> {
    ctx: &'a mut Ctx,
    dryrun: bool,
    soft_revert: bool,
    start_idx: Option<u32>,
}

impl<'a, Ctx> Down<'a, Ctx> {
    /// New `Revert` operation.
    pub fn new_revert(ctx: &'a mut Ctx) -> Self {
        Self { ctx, dryrun: false, soft_revert: false, start_idx: None }
    }

    /// New `Revert` removing from history but skipping the migration query.
    pub fn new_soft_revert(ctx: &'a mut Ctx) -> Self {
        Self { ctx, dryrun: false, soft_revert: true, start_idx: None }
    }

    /// Set this to be a dryrun.
    pub fn dryrun(self) -> Down<'a, Ctx> {
        Down {
            ctx: self.ctx,
            dryrun: true,
            soft_revert: self.soft_revert,
            start_idx: None,
        }
    }

    /// Set the index of the statement with the first migration the start from.
    pub fn start_idx(self, idx: u32) -> Down<'a, Ctx> {
        Down {
            ctx: self.ctx,
            dryrun: true,
            soft_revert: self.soft_revert,
            start_idx: Some(idx),
        }
    }

    /// `true` if this is a dryrun.
    pub fn is_dryrun(&self) -> bool {
        self.dryrun
    }

    /// `true` if only the history table will update.
    pub fn is_soft_revert(&self) -> bool {
        self.soft_revert
    }

    /// Gets the index to start from.
    pub fn get_start_idx(&self) -> Option<u32> {
        self.start_idx
    }
}

impl<Ctx: MigrationContext> MigrationOp<&DownMigration<Ctx>> for Down<'_, Ctx> {
    type Output = TernResult<MigrationData>;

    async fn exec(self, input: &DownMigration<Ctx>) -> Self::Output {
        let id = input.migration_id();
        let start = Utc::now();

        let applied = async {
            // Override this to run outside a transaction.
            // This is the opinionated behavior of `tern` with down migrations.
            let q = RenderQuery::new(self.ctx).exec(input).await?.force_notx();
            if self.is_dryrun() {
                return Ok::<_, TernError>(MigrationData::new(id, &q, start));
            }
            if !self.is_soft_revert() {
                self.ctx.executor_mut().send(&q).await?;
                log::debug!(id:%; "reverted migration");
            }

            let applied = MigrationData::new(id, &q, start);
            DeleteApplied::new(self.ctx).exec(&applied).await?;

            Ok(applied)
        }
        .await
        .map_err_id(id)?;

        Ok(applied)
    }
}
