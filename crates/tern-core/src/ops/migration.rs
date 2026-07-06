//! Operations to apply migrations.
use chrono::Utc;
use std::marker::PhantomData;

use crate::context::{MigrationContext, MigrationExecutor as _};
use crate::error::{ResultWithId as _, TernError, TernResult};
use crate::migration::{Migration, MigrationData};
use crate::ops::Operation;
use crate::ops::crud::{DeleteApplied, UpdateApplied};
use crate::query::Query;

/// Operation to statically return the query of a migration.
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

impl<M, Ctx> Operation<&M> for StaticQuery<'_, Ctx>
where
    Ctx: MigrationContext,
    M: Migration<Ctx = Ctx>,
{
    type Output = Query;

    async fn exec(self, input: &M) -> Self::Output {
        let id = input.migration_id();
        let q = input.show_query();
        log::debug!(id:%, query:% = q; "static query");
        q
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

impl<M, Ctx> Operation<&M> for RenderQuery<'_, Ctx>
where
    Ctx: MigrationContext,
    M: Migration<Ctx = Ctx>,
{
    type Output = TernResult<Query>;

    async fn exec(self, input: &M) -> Self::Output {
        let id = input.migration_id();
        let q = input.query(self.0).await?;
        log::debug!(id:%, query:% = q; "rendered query");
        Ok(q)
    }
}

/// An operation for applying one migration and updating the history table on
/// success.
#[derive(Debug)]
pub struct ApplyOne<'a, Ctx> {
    ctx: &'a mut Ctx,
    dryrun: bool,
    soft_apply: bool,
}

impl<'a, Ctx> ApplyOne<'a, Ctx> {
    /// New `ApplyOne` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self { ctx, dryrun: false, soft_apply: false }
    }

    /// Set this to be a "soft" apply.
    pub fn soft_apply(self) -> ApplyOne<'a, Ctx> {
        ApplyOne { ctx: self.ctx, dryrun: true, soft_apply: true }
    }

    /// Set this to be a dryrun.
    pub fn dryrun(self) -> ApplyOne<'a, Ctx> {
        ApplyOne { ctx: self.ctx, dryrun: true, soft_apply: self.soft_apply }
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

impl<M, Ctx> Operation<&M> for ApplyOne<'_, Ctx>
where
    Ctx: MigrationContext,
    M: Migration<Ctx = Ctx>,
{
    type Output = TernResult<MigrationData>;

    async fn exec(self, input: &M) -> Self::Output {
        let id = input.migration_id();
        let version = id.version();
        let description = id.description();
        let start = Utc::now();
        let q = RenderQuery(self.ctx).exec(input).await?;

        let mut data = MigrationData::new(version, description, &q);

        if self.dryrun {
            data.finished(start);
            return Ok::<_, TernError>(data);
        }
        if !self.soft_apply {
            self.ctx.executor_mut().send(&q).await?;
            log::debug!(id:%; "applied migration");
        }

        data.finished(start);
        UpdateApplied::new(self.ctx).exec(&data).await.map_err_id(id)?;

        Ok(data)
    }
}

/// An operation for applying down migration and updating the history table on
/// success.
#[derive(Debug)]
pub struct RevertOne<'a, Ctx> {
    ctx: &'a mut Ctx,
    dryrun: bool,
    soft_revert: bool,
}

impl<'a, Ctx> RevertOne<'a, Ctx> {
    /// New `Revert` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self { ctx, dryrun: false, soft_revert: false }
    }

    /// Set this to be a "soft" revert.
    pub fn soft_revert(self) -> RevertOne<'a, Ctx> {
        RevertOne { ctx: self.ctx, dryrun: true, soft_revert: true }
    }

    /// Set this to be a dryrun.
    pub fn dryrun(self) -> RevertOne<'a, Ctx> {
        RevertOne { ctx: self.ctx, dryrun: true, soft_revert: self.soft_revert }
    }

    /// `true` if this is a dryrun.
    pub fn is_dryrun(&self) -> bool {
        self.dryrun
    }

    /// `true` if only the history table will update.
    pub fn is_soft_revert(&self) -> bool {
        self.soft_revert
    }
}

impl<M, Ctx> Operation<&M> for RevertOne<'_, Ctx>
where
    Ctx: MigrationContext,
    M: Migration<Ctx = Ctx>,
{
    type Output = TernResult<MigrationData>;

    async fn exec(self, input: &M) -> Self::Output {
        let id = input.migration_id();
        let version = id.version();
        let description = id.description();
        let start = Utc::now();
        let q = RenderQuery(self.ctx).exec(input).await?;

        let mut data = MigrationData::new(version, description, &q);

        if self.dryrun {
            data.finished(start);
            return Ok::<_, TernError>(data);
        }
        if !self.soft_revert {
            self.ctx.executor_mut().send(&q).await?;
            log::debug!(id:%; "reverted migration");
        }

        data.finished(start);
        DeleteApplied::new(self.ctx).exec(&data).await.map_err_id(id)?;

        Ok(data)
    }
}
