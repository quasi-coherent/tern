//! The CLI for the [`tern`][tern-docs] migration library.
//!
//! This exports the [`App`] type and [`ContextOptions`], which help turn a
//! project using `tern` into a CLI.
//!
//! The `App` is the CLI. `ContextOptions` helps to connect a generic context to
//! the CLI if it is the CLI that is supplying the database connection string.
//!
//! [tern-docs]: https://docs.rs/crate/tern/latest
use clap::Parser;
use tern_core::error::TernResult;
use tern_core::future::Future;
use tern_core::migration::MigrationContext;
use tern_core::runner::{Report, Runner};

mod cli;
mod commands;

/// A type that can build a particular context given a database url.
pub trait ContextOptions {
    type Ctx: MigrationContext;

    /// Establish a connection with this context.
    fn connect(&self, db_url: &str) -> impl Future<Output = TernResult<Self::Ctx>>;
}

/// The CLI app to run.
///
/// ## Usage
///
/// Either build from [`ContextOptions`] and supply the database connection
/// string with the CLI and `-D`, `--database-url`, or environment variable
/// `DATABASE_URL`, or build `App` directly from a `MigrationContext`.
///
/// ```terminal
/// > $ my-app --help
/// Usage: my-app <COMMAND>
///
/// Commands:
///   migrate  Operations on the set of migration files
///   history  Operations on the table storing the history of these migrations
///   help     Print this message or the help of the given subcommand(s)
/// ```
pub struct App<T> {
    inner: T,
    cli: cli::Tern,
}

impl<T> App<T> {
    pub fn new(inner: T) -> Self {
        let cli = cli::Tern::parse();
        Self { inner, cli }
    }

    /// Run a CLI that has a `T: ContextOptions`, using the context that these
    /// options can build.
    pub async fn run(&self) -> anyhow::Result<Option<Report>>
    where
        T: ContextOptions,
    {
        match &self.cli.commands {
            cli::TernCommands::History(history) => match &history.commands {
                cli::HistoryCommands::Init { connect_opts } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.inner.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    runner.init_history().await?;

                    Ok(None)
                }
                cli::HistoryCommands::Drop { connect_opts } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.inner.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    runner.drop_history().await?;

                    Ok(None)
                }
                cli::HistoryCommands::SoftApply { .. } => Err(anyhow::anyhow!(
                    "Deprecated: use `migrate soft-apply` instead"
                )),
            },
            cli::TernCommands::Migrate(migrate) => match &migrate.commands {
                cli::MigrateCommands::Apply {
                    dryrun,
                    target_version,
                    connect_opts,
                } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.inner.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    let report = runner.run_apply(*target_version, *dryrun).await?;

                    Ok(Some(report))
                }
                cli::MigrateCommands::ApplyAll {
                    dryrun,
                    connect_opts,
                } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.inner.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    let report = runner.run_apply_all(*dryrun).await?;

                    Ok(Some(report))
                }
                cli::MigrateCommands::SoftApply {
                    dryrun,
                    target_version,
                    connect_opts,
                } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.inner.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    let report = runner.run_soft_apply(*target_version, *dryrun).await?;

                    Ok(Some(report))
                }
                cli::MigrateCommands::ListApplied { connect_opts } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.inner.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    let report = runner.list_applied().await?;

                    Ok(Some(report))
                }
                cli::MigrateCommands::New {
                    description,
                    no_tx,
                    migration_type,
                    source,
                } => {
                    commands::new(
                        description.to_string(),
                        *no_tx,
                        *migration_type,
                        source.path.clone(),
                    )?;

                    Ok(None)
                }
            },
        }
    }

    /// Run the CLI having already built a `MigrationContext` and initialized the
    /// `App` from it instead of builder options.
    pub async fn run_with_context(self) -> anyhow::Result<Option<Report>>
    where
        T: MigrationContext,
    {
        let mut runner = Runner::new(self.inner);
        let cli = self.cli;

        match cli.commands {
            cli::TernCommands::History(history) => match &history.commands {
                cli::HistoryCommands::Init { .. } => {
                    runner.init_history().await?;

                    Ok(None)
                }
                cli::HistoryCommands::Drop { .. } => {
                    runner.drop_history().await?;

                    Ok(None)
                }
                cli::HistoryCommands::SoftApply { .. } => Err(anyhow::anyhow!(
                    "Deprecated: use `migrate soft-apply` instead"
                )),
            },
            cli::TernCommands::Migrate(migrate) => match migrate.commands {
                cli::MigrateCommands::Apply {
                    dryrun,
                    target_version,
                    ..
                } => {
                    let report = runner.run_apply(target_version, dryrun).await?;

                    Ok(Some(report))
                }
                cli::MigrateCommands::ApplyAll { dryrun, .. } => {
                    let report = runner.run_apply_all(dryrun).await?;

                    Ok(Some(report))
                }
                cli::MigrateCommands::SoftApply {
                    dryrun,
                    target_version,
                    ..
                } => {
                    let report = runner.run_soft_apply(target_version, dryrun).await?;

                    Ok(Some(report))
                }
                cli::MigrateCommands::ListApplied { .. } => {
                    let report = runner.list_applied().await?;

                    Ok(Some(report))
                }
                cli::MigrateCommands::New {
                    description,
                    no_tx,
                    migration_type,
                    source,
                } => {
                    commands::new(
                        description.to_string(),
                        no_tx,
                        migration_type,
                        source.path.clone(),
                    )?;

                    Ok(None)
                }
            },
        }
    }
}
