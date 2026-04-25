//! CRUD operations on the records of the history table.
use crate::context::{MigrationContext, MigrationExecutor};
use crate::error::TernResult;
use crate::migration::MigrationData;
use crate::ops::MigrationOp;

/// An operation for inserting an applied migration into the history table.
#[derive(Debug)]
pub struct InsertApplied<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> InsertApplied<'a, Ctx> {
    /// New `InsertApplied` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> MigrationOp<&MigrationData>
    for InsertApplied<'_, Ctx>
{
    type Output = TernResult<()>;

    async fn exec(self, input: &MigrationData) -> Self::Output {
        let history = self.0.history_table();
        self.0.executor_mut().insert_applied(history, input).await?;
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

impl<'a, F, Ctx> MigrationOp<F> for ReadAppliedWhere<'a, Ctx>
where
    Ctx: MigrationContext,
    F: FnMut(&MigrationData) -> bool + Send + 'a,
{
    type Output = TernResult<Vec<MigrationData>>;

    async fn exec(self, args: F) -> Self::Output {
        let history = self.0.history_table();
        let applied = self
            .0
            .executor_mut()
            .get_all_applied(history)
            .await?
            .into_iter()
            .filter(args)
            .collect();
        Ok(applied)
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

impl<Ctx: MigrationContext> MigrationOp<&MigrationData>
    for UpdateApplied<'_, Ctx>
{
    type Output = TernResult<()>;

    async fn exec(self, input: &MigrationData) -> Self::Output {
        let history = self.0.history_table();
        self.0.executor_mut().upsert_applied(history, input).await?;
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

impl<Ctx: MigrationContext> MigrationOp<&MigrationData>
    for DeleteApplied<'_, Ctx>
{
    type Output = TernResult<()>;

    async fn exec(self, input: &MigrationData) -> Self::Output {
        let version = input.version();
        let history = self.0.history_table();
        self.0.executor_mut().delete_applied(history, version).await?;
        log::debug!(version:%; "deleted applied");
        Ok(())
    }
}
