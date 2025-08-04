use chrono::Utc;
use tern_core::context::MigrationContext;
use tern_core::error::{DatabaseError as _, TernResult};
use tern_core::source::MigrationSet;

#[cfg(feature = "cli")]
mod cli_opts;

mod report;
pub use report::{AttachReport as _, MigrationResult, Report};

mod validate;
use validate::Validator;

/// `TernOp` are the operations exposed by `Tern` for a migration context.
#[derive(Debug, Clone, Copy, Default)]
pub enum TernOp {
    /// List all applied migrations.
    #[default]
    ListApplied,
    /// Apply all unapplied migrations.
    ApplyAll,
    /// Apply unapplied migrations, up to and including, the target version.
    ApplyThrough(i64),
    /// Do a "soft apply" of all unapplied migrations.
    ///
    /// This creates a record in the history table for the migration as if it
    /// were applied, but does not run the query for the migration.  This can be
    /// useful to sync history with the state of the database or to migrate a
    /// history table.
    SoftApplyAll,
    /// Soft apply unapplied migrations, up to and including, the target version.
    SoftApplyThrough(i64),
    /// Create the migration history table.
    InitHistory,
    /// Drop the migration history table.
    DropHistory,
}

/// `Tern` is the main application wrapping a given `MigrationContext` and
/// exposing the available operations.
#[derive(Debug, Clone)]
pub struct Tern<Ctx> {
    context: Ctx,
    dryrun: bool,
    operation: TernOp,
}

impl<Ctx: MigrationContext> Tern<Ctx> {
    /// Create a new `Tern` application with default options from the given
    /// `MigrationContext`.
    pub fn new(context: Ctx) -> Self {
        Self {
            context,
            dryrun: false,
            operation: TernOp::default(),
        }
    }

    /// Run this `Tern` application.
    pub async fn run(&mut self) -> TernResult<Option<Report>> {
        let target = match self.operation {
            TernOp::InitHistory => {
                log::trace!(target: "tern", "init history table {}", Ctx::HISTORY_TABLE);
                self.init_history().await?;
                return Ok(None);
            }
            TernOp::DropHistory => {
                log::trace!(target: "tern", "drop history table {}", Ctx::HISTORY_TABLE);
                self.drop_history().await?;
                return Ok(None);
            }
            TernOp::ListApplied => {
                log::trace!(target: "tern", "list applied {}", Ctx::HISTORY_TABLE);
                let report = self.list_applied().await?;
                return Ok(Some(report));
            }
            TernOp::ApplyAll | TernOp::SoftApplyAll => None,
            TernOp::ApplyThrough(n) | TernOp::SoftApplyThrough(n) => Some(n),
        };

        log::trace!(target: "tern", "validate");
        let mut validator = Validator::new(&mut self.context);
        validator.validate(target).await?;

        // Get the correct subset of migrations: version greater than the last
        // applied and less than or equal to target.
        //
        // This will attempt to acquire the migration lock before calculating the
        // migration set.  If this process wasn't the first to acquire the lock,
        // the migration set returned will be empty because the last applied is
        // the target version.
        let migration_set = self.get_migration_set(target).await?;

        if self.operation.is_apply() {
            self.apply(self.dryrun, migration_set).await.map(Some)
        } else if self.operation.is_soft_apply() {
            self.soft_apply(self.dryrun, migration_set).await.map(Some)
        } else {
            Ok(None)
        }
    }

    /// Return a report of what the operation would do if ran without `dryrun`.
    pub fn dryrun(mut self) -> Self {
        self.dryrun = true;
        self
    }

    /// Set the operation that should be ran with this configuration.
    pub fn with_operation(mut self, op: TernOp) -> Self {
        self.operation = op;
        self
    }

    async fn init_history(&mut self) -> TernResult<()> {
        self.context.check_history_table().await
    }

    async fn drop_history(&mut self) -> TernResult<()> {
        self.context.drop_history_table().await
    }

    async fn list_applied(&mut self) -> TernResult<Report> {
        let applied = self
            .context
            .previously_applied()
            .await?
            .iter()
            .map(|applied| MigrationResult::from_applied(applied, None))
            .collect();

        let report = Report::new(applied);

        Ok(report)
    }

    /// Get the set of migrations for the operation.
    ///
    /// This is source migrations with version greater than the last applied and
    /// less than or equal to the target.
    async fn get_migration_set(&mut self, target: Option<i64>) -> TernResult<MigrationSet<Ctx>> {
        // All source migrations (applied or not) through the target version.
        let set = self.context.migration_set(target);
        let Some(latest) = self.context.latest_version().await? else {
            return Ok(set);
        };

        // Now get source migrations that have a version greater than the last
        // applied migration.
        let unapplied = set
            .migrations
            .into_iter()
            .filter(|m| m.as_ref().version() > latest)
            .collect::<Vec<_>>();

        Ok(MigrationSet::new(unapplied))
    }

    async fn apply(&mut self, dryrun: bool, set: MigrationSet<Ctx>) -> TernResult<Report> {
        let mut results = Vec::new();

        for migration in &set.migrations {
            let id = migration.migration_id();

            if dryrun {
                // Not calling context's `apply` in a dry run, so the query needs
                // to be built, or otherwise don't go so far as to build the
                // query in a dry run.
                let query = migration
                    .build(&mut self.context)
                    .await
                    .with_report(&results)?;

                let result = MigrationResult::from_unapplied(migration.as_ref(), query.sql());
                results.push(result);

                continue;
            }

            log::trace!(target: "tern", "applying migration {id}");
            let result = self
                .context
                .apply(migration.as_ref())
                .await
                .tern_migration_result(migration.as_ref())
                .with_report(&results)
                .map(|v| MigrationResult::from_applied(&v, Some(migration.no_tx())))?;

            results.push(result);
        }

        Ok(Report::new(results))
    }

    async fn soft_apply(&mut self, dryrun: bool, set: MigrationSet<Ctx>) -> TernResult<Report> {
        let mut results = Vec::new();

        for migration in &set.migrations {
            let id = migration.migration_id();

            let query = migration
                .build(&mut self.context)
                .await
                .with_report(&results)?;

            let applied = migration.to_applied(0, Utc::now(), query.sql());
            let result = MigrationResult::from_soft_applied(&applied, dryrun);

            if !dryrun {
                log::trace!(target: "tern", "soft applying migration {id}");
                self.context
                    .insert_applied(&applied)
                    .await
                    .with_report(&results)?;
            }

            results.push(result);
        }

        Ok(Report::new(results))
    }
}

impl TernOp {
    fn is_apply(&self) -> bool {
        matches!(self, TernOp::ApplyAll | TernOp::ApplyThrough(_))
    }

    fn is_soft_apply(&self) -> bool {
        matches!(self, TernOp::SoftApplyAll | TernOp::SoftApplyThrough(_))
    }
}
