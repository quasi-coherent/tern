//! Operations with history table DDL.
use crate::context::{MigrationContext, MigrationExecutor};
use crate::error::{TernError, TernResult};
use crate::ops::MigrationOp;

/// An operation for creating a `tern` project and history table.
#[derive(Debug)]
pub struct Init<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> Init<'a, Ctx> {
    /// New `Init` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> MigrationOp<()> for Init<'_, Ctx> {
    type Output = TernResult<()>;

    async fn exec(self, _: ()) -> Self::Output {
        let history = self.0.history_table();
        // Check if the history table already exists:
        if self.0.executor_mut().check_history(history).await.is_ok() {
            log::error!(namespace:% = history; "table already exists");
            return Err(TernError::History(
                "init failed: history table exists",
            ))?;
        }
        self.0.executor_mut().init_history(history).await?;
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

impl<Ctx: MigrationContext> MigrationOp<()> for DropIfExists<'_, Ctx> {
    type Output = TernResult<()>;

    async fn exec(self, _input: ()) -> Self::Output {
        let history = self.0.history_table();
        // Check if the history table exists:
        if self.0.executor_mut().check_history(history).await.is_ok() {
            log::warn!(namespace:% = history; "history table does not exist");
            return Ok(());
        }
        self.0.executor_mut().drop_history(history).await?;
        log::debug!(namespace:% = history; "dropped history table");
        Ok(())
    }
}
