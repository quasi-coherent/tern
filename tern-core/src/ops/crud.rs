//! CRUD operations on the records of the history table.
use crate::context::{MigrationContext, MigrationExecutor as _};
use crate::error::TernResult;
use crate::migration::MigrationData;
use crate::ops::Operation;

/// An operation for inserting an applied migration into the history table.
#[derive(Debug)]
pub struct InsertApplied<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> InsertApplied<'a, Ctx> {
    /// New `InsertApplied` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> Operation<&MigrationData>
    for InsertApplied<'_, Ctx>
{
    type Output = TernResult<()>;

    async fn exec(self, input: &MigrationData) -> Self::Output {
        let history = self.0.history_table();
        self.0.executor_mut().insert_into(history, input).await?;
        log::debug!(version:% = input.version(); "inserted applied");
        Ok(())
    }
}

/// An operation for retrieving applied migrations satisfying a given condition.
#[derive(Debug)]
pub struct ReadAppliedWhere<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> ReadAppliedWhere<'a, Ctx> {
    /// New `ReadAppliedWhere` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<'a, Ctx> Operation<(Option<i64>, Option<i64>)>
    for ReadAppliedWhere<'a, Ctx>
where
    Ctx: MigrationContext,
{
    type Output = TernResult<Vec<MigrationData>>;

    async fn exec(
        self,
        (min_version, max_version): (Option<i64>, Option<i64>),
    ) -> Self::Output {
        let history = self.0.history_table();
        self.0
            .executor_mut()
            .select_where_version_between(history, min_version, max_version)
            .await
    }
}

/// An operation for updating an applied migration in the history table.
#[derive(Debug)]
pub struct UpdateApplied<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> UpdateApplied<'a, Ctx> {
    /// New `UpdateApplied` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> Operation<&MigrationData>
    for UpdateApplied<'_, Ctx>
{
    type Output = TernResult<()>;

    async fn exec(self, input: &MigrationData) -> Self::Output {
        let history = self.0.history_table();
        self.0.executor_mut().insert_or_update(history, input).await?;
        log::debug!(version:% = input.version(); "updated applied");
        Ok(())
    }
}

/// An operation for removing an applied migration from the history table.
#[derive(Debug)]
pub struct DeleteApplied<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> DeleteApplied<'a, Ctx> {
    /// New `DeleteApplied` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> Operation<&MigrationData>
    for DeleteApplied<'_, Ctx>
{
    type Output = TernResult<()>;

    async fn exec(self, input: &MigrationData) -> Self::Output {
        let version = input.version();
        let history = self.0.history_table();
        self.0.executor_mut().delete_from(history, version).await?;
        log::debug!(version:%; "deleted applied");
        Ok(())
    }
}
