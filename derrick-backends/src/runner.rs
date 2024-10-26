use derrick_core::error::Error;
use derrick_core::prelude::*;
use derrick_core::reexport::BoxFuture;
use derrick_core::types::{
    AppliedMigration, FutureMigration, HistoryTableInfo, Migration, MigrationSource,
};
use std::borrow::Cow;

/// Migration runner -- it interacts with
/// methods created by proc macro code that
/// prepare migration sources or final, unapplied
/// migration sets.
pub struct Runner<'a> {
    table_info: HistoryTableInfo,
    unapplied: Cow<'a, [Migration]>,
}

impl<'a> Runner<'a> {
    pub fn new(table_info: HistoryTableInfo) -> Self {
        Self {
            table_info,
            unapplied: Cow::Owned(Vec::new()),
        }
    }

    /// Validates a migration set.  Returns the current version.
    pub async fn ready<M>(
        &self,
        migrate: &mut M,
        sources: Vec<MigrationSource>,
    ) -> Result<i64, Error>
    where
        M: Migrate,
    {
        let table = M::Table::new(&self.table_info);

        migrate.check_history_table(&table).await?;

        let applied = migrate.get_history_table(&table).await?;
        let latest = Self::last_applied(&applied).unwrap_or_default();

        M::validate_source(sources, applied)?;

        Ok(latest)
    }

    /// Applied the migration set.  Returns the report.
    pub async fn run<M>(&self, migrate: &mut M) -> Result<Vec<AppliedMigration>, Error>
    where
        M: Migrate,
    {
        let table = M::Table::new(&self.table_info);
        let unapplied = &self.unapplied;
        log::info!(migrations:? = unapplied, table:% = table.table(); "applying migration set");

        let mut applied = Vec::new();
        for migration in unapplied.into_iter() {
            let new_applied = migrate.apply(&table, &migration).await?;
            applied.push(new_applied);
        }

        Ok(applied)
    }

    pub fn set_unapplied(mut self, unapplied: Vec<Migration>) -> Self {
        self.unapplied = Cow::Owned(unapplied);
        self
    }

    fn last_applied(applied: &[AppliedMigration]) -> Option<i64> {
        let mut latest: Option<i64> = None;
        for m in applied.iter() {
            match latest {
                None => latest = Some(m.version),
                Some(v) if m.version > v => latest = Some(m.version),
                _ => (),
            }
        }

        latest
    }
}
