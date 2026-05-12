use std::fmt::{self, Display, Formatter};

use crate::migrate::TernMigrate;
use crate::operation::TernMigrateOp;
use crate::report::Report;

/// Initialize the migration history table.
#[derive(Debug, Clone, Copy, Default)]
pub struct Init;

impl<T: TernMigrate> TernMigrateOp<T> for Init {
    type Output = ();

    async fn exec(
        &self,
        migrate: &mut T,
    ) -> Report<Self::Output> {
        if migrate.check_history_exists().await.is_ok() {
            log::warn!("History table already exists!");
            return Ok(());
        }
        migrate.init_history().await?;
        Ok(())
    }
}

impl Display for Init {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "InitHistory")
    }
}

/// Drop the migration history table.
#[derive(Debug, Clone, Copy, Default)]
pub struct Drop;

impl<T: TernMigrate> TernMigrateOp<T> for Drop {
    type Output = ();

    async fn exec(
        &self,
        migrate: &mut T,
    ) -> Report<Self::Output> {
        migrate.check_history_exists().await.inspect_err(|_| {
            log::error!("Drop failed: history table not found");
        })?;
        migrate.drop_history().await?;
        Ok(())
    }
}

impl Display for Drop {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "DropHistory")
    }
}
