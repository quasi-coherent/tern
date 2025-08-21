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

/// `TernBuilder` builds a runnable [Tern] migration app.
#[derive(Debug, Clone, Copy, Default)]
pub struct TernBuilder {
    dry_run: bool,
    skip_validate: bool,
    target: Option<i64>,
    op: TernOp,
}

impl TernBuilder {
    /// Return a report detailing what would have been done if it were not a
    /// dry run.
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Do not run validation of the local source migrations with the history
    /// table before the operation.
    pub fn skip_validate(mut self) -> Self {
        self.skip_validate = true;
        self
    }

    /// Where applicable, only do the operation up to and including the target
    /// version.
    pub fn with_target_version(mut self, target: i64) -> Self {
        self.target = Some(target);
        self
    }

    /// Set the operation to initialize the history table.
    pub fn init_history(mut self) -> Self {
        self.op = HistoryOp::Init.into();
        self
    }

    /// Set the operation to drop the history table.
    pub fn drop_history(mut self) -> Self {
        self.op = HistoryOp::Drop.into();
        self
    }

    /// Set the operation to list migrations that have been applied.
    pub fn list_applied(mut self) -> Self {
        self.op = MigrateOp::ListApplied.into();
        self
    }

    /// Set the operation to apply unapplied migrations with this configuration.
    pub fn apply(mut self) -> Self {
        self.op = MigrateOp::Apply.into();
        self
    }

    /// Set the operation to "soft" apply unapplied migrations with this
    /// configuration's target version.
    ///
    /// A soft apply is one where a migration's query is built and a record is
    /// created in the history table, but the query is not actually ran.  This
    /// can be useful when syncing migrations with the state of a database or
    /// when migrating history tables.
    pub fn soft_apply(mut self) -> Self {
        self.op = MigrateOp::SoftApply.into();
        self
    }

    /// Build the [Tern] app from this configuration for the given context.
    pub fn build_with_context<Ctx: MigrationContext>(self, context: Ctx) -> Tern<Ctx> {
        Tern {
            context,
            dry_run: self.dry_run,
            skip_validate: self.skip_validate,
            target: self.target,
            op: self.op,
        }
    }
}

/// `Tern` is the main application wrapping a given [MigrationContext] and
/// exposing the available operations.
///
/// [MigrationContext]: tern_core::context::MigrationContext
#[derive(Debug, Clone)]
pub struct Tern<Ctx = ()> {
    context: Ctx,
    dry_run: bool,
    skip_validate: bool,
    target: Option<i64>,
    op: TernOp,
}

impl Tern {
    pub fn builder() -> TernBuilder {
        TernBuilder::default()
    }
}

impl<Ctx: MigrationContext> Tern<Ctx> {
    /// Run this [Tern] application with the given configuration.
    pub async fn run(&mut self) -> TernResult<Option<Report>> {
        match self.op {
            TernOp::History(history) => {
                self.run_history(history).await?;
                Ok(None)
            }
            TernOp::Migrate(migrate) => self.run_migrate(migrate).await.map(Some),
        }
    }

    async fn run_history(&mut self, op: HistoryOp) -> TernResult<()> {
        match op {
            HistoryOp::Init => {
                log::trace!(target: "tern", "init history table {}", Ctx::HISTORY_TABLE);
                self.context.check_history_table().await
            }
            HistoryOp::Drop => {
                log::trace!(target: "tern", "drop history table {}", Ctx::HISTORY_TABLE);
                self.context.drop_history_table().await
            }
        }
    }

    async fn run_migrate(&mut self, op: MigrateOp) -> TernResult<Report> {
        if !self.skip_validate {
            log::trace!(target: "tern", "validate");
            let mut validator = Validator::new(&mut self.context);
            validator.validate(self.target).await?;
        }

        // Get the correct subset of migrations: version greater than the last
        // applied and less than or equal to target.
        let migration_set = self.get_migration_set(self.target).await?;

        match op {
            MigrateOp::ListApplied => self.list_applied().await,
            MigrateOp::Apply => self.apply(self.dry_run, migration_set).await,
            MigrateOp::SoftApply => self.soft_apply(self.dry_run, migration_set).await,
        }
    }

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

/// The subcommands of `TernApp`.
#[derive(Debug, Clone, Copy)]
enum TernOp {
    Migrate(MigrateOp),
    History(HistoryOp),
}

impl Default for TernOp {
    fn default() -> Self {
        Self::Migrate(MigrateOp::ListApplied)
    }
}

/// `MigrateOp` are the possible migration operations.
#[derive(Debug, Clone, Copy, Default)]
enum MigrateOp {
    #[default]
    ListApplied,
    Apply,
    SoftApply,
}

/// HistoryOp`` are the possible history table operations.
#[derive(Debug, Clone, Copy, Default)]
enum HistoryOp {
    #[default]
    Init,
    Drop,
}

impl From<MigrateOp> for TernOp {
    fn from(value: MigrateOp) -> Self {
        Self::Migrate(value)
    }
}

impl From<HistoryOp> for TernOp {
    fn from(value: HistoryOp) -> Self {
        Self::History(value)
    }
}
