//! The CLI for the [`tern`][tern-docs] migration library.
//!
//! This exports the [`App`] type and [`ContextOptions`], which help turn a
//! project using `tern` into a CLI.
//!
//! The `App` is the CLI. `ContextOptions` exists to connect a generic context
//! to the CLI since it is the CLI that supplies the database URL, surely
//! required of the context, but not anything else the context might need to
//! initialize.
//!
//! [tern-docs]: https://docs.rs/crate/tern/latest
use clap::Parser;
use tern_core::error::TernResult;
use tern_core::future::Future;
use tern_core::migration::MigrationContext;
use tern_core::runner::Runner;

mod cli;
mod commands;

/// A type that can build a particular context given a database url.
/// This is needed because the context is arbitrary, yet the CLI options have
/// the database URL, which is certainly required to build it.
pub trait ContextOptions {
    type Ctx: MigrationContext;

    /// Establish a connection with this context.
    fn connect(&self, db_url: &str) -> impl Future<Output = TernResult<Self::Ctx>>;
}

/// The CLI app to run.  This wraps the functionality of the context that `Opts`
/// creates.
///
/// ## Usage
///
/// To connect to the given database, the CLI needs a database url, which can be
/// provided via the environment variable `DATABASE_URL` or using the option
/// `-D/--database-url` available to a command/subcommand.
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
pub struct App<Opts> {
    opts: Opts,
    cli: cli::Tern,
}

impl<Opts> App<Opts>
where
    Opts: ContextOptions,
{
    pub fn new(opts: Opts) -> Self {
        let cli = cli::Tern::parse();
        Self { opts, cli }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        match &self.cli.commands {
            cli::TernCommands::History(history) => match &history.commands {
                cli::HistoryCommands::Init { connect_opts } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.opts.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    runner.init_history().await?;

                    Ok(())
                }
                cli::HistoryCommands::Drop { connect_opts } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.opts.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    runner.drop_history().await?;

                    Ok(())
                }
                cli::HistoryCommands::SoftApply {
                    from_version,
                    to_version,
                    connect_opts,
                } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.opts.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    let report = runner.soft_apply(*from_version, *to_version).await?;
                    log::info!("{report:#?}");

                    Ok(())
                }
            },
            cli::TernCommands::Migrate(migrate) => match &migrate.commands {
                cli::MigrateCommands::ApplyAll {
                    dryrun,
                    connect_opts,
                } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.opts.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    let report = if *dryrun {
                        runner.dryrun().await?
                    } else {
                        runner.apply_all().await?
                    };
                    log::info!("{report:#?}");

                    Ok(())
                }
                cli::MigrateCommands::ListApplied { connect_opts } => {
                    let db_url = connect_opts.required_db_url()?.to_string();
                    let context = self.opts.connect(&db_url).await?;
                    let mut runner = Runner::new(context);
                    let report = runner.list_applied().await?;
                    log::info!("{report:#?}");

                    Ok(())
                }
                cli::MigrateCommands::New {
                    description,
                    no_tx,
                    migration_type,
                    source,
                } => commands::new(
                    description.to_string(),
                    *no_tx,
                    *migration_type,
                    source.path.clone(),
                ),
            },
        }
    }
}
