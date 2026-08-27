//! Operations with history table DDL.
use crate::context::{HistoryRelid, MigrationContext, MigrationExecutor as _};
use crate::error::{TernError, TernResult};
use crate::ops::Operation;

/// An operation for creating a `tern` project and history table.
#[derive(Debug)]
pub struct CreateIfNotExists<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> CreateIfNotExists<'a, Ctx> {
    /// New `CreateIfNotExists` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> Operation<HistoryRelid>
    for CreateIfNotExists<'_, Ctx>
{
    type Output = TernResult<()>;

    async fn exec(self, history: HistoryRelid) -> Self::Output {
        let exec = self.0.executor_mut();
        if exec.history_exists(history).await.is_ok_and(std::convert::identity)
        {
            return Err(TernError::History(
                "new history failed: history table exists",
            ))?;
        };
        exec.create_if_not_exists(history).await?;
        log::debug!(namespace:% = history; "created history table");
        Ok(())
    }
}

/// An operation dropping the history table.
#[derive(Debug)]
pub struct DropIfExists<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> DropIfExists<'a, Ctx> {
    /// New `Drop` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> Operation<HistoryRelid> for DropIfExists<'_, Ctx> {
    type Output = TernResult<()>;

    async fn exec(self, history: HistoryRelid) -> Self::Output {
        let exec = self.0.executor_mut();
        if !exec.history_exists(history).await? {
            log::warn!(namespace:% = history; "history table does not exist");
            return Ok(());
        }
        exec.drop_if_exists(history).await?;
        log::debug!(namespace:% = history; "dropped history table");
        Ok(())
    }
}
