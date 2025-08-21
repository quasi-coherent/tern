use clap::{Args, Parser};
use futures_core::Future;
use std::fmt::Debug;
use tern_core::context::MigrationContext;
use tern_core::error::TernResult;

pub extern crate clap;

/// A type that can initialize a [MigrationContext] from command line arguments.
///
/// [MigrationContext]: tern_core::context::MigrationContext
pub trait ConnectOptions: Args + Debug {
    /// The target context for this type.
    type Ctx: MigrationContext;

    /// Connect to the backend and create the migration context.
    fn connect(&self) -> impl Future<Output = TernResult<Self::Ctx>>;
}

/// Command-line interface for running a `tern` migration application
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct CliOpts<C: ConnectOptions> {
    #[clap(flatten)]
    pub connect_opts: C,
    #[clap(subcommand)]
    pub opts: Opts,
}

impl<C: ConnectOptions> Default for CliOpts<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: ConnectOptions> CliOpts<C> {
    /// Create a new [CliOpts] by parsing command line options.
    pub fn new() -> Self {
        Self::parse()
    }
}

/// Subcommands of the `tern` CLI.
#[derive(Debug, Parser)]
pub enum Opts {
    Migrate(Migrate),
    History(History),
}

/// Operations on the set of source migration files
#[derive(Debug, Parser)]
pub struct Migrate {
    #[clap(subcommand)]
    pub opts: MigrateOpts,
}

/// Operations on the table storing the history of these migrations
#[derive(Debug, Parser)]
pub struct History {
    #[clap(subcommand)]
    pub opts: HistoryOpts,
}

#[derive(Debug, Parser)]
pub enum MigrateOpts {
    /// Run the apply operation for all unapplied versions or a range of versions
    Apply {
        /// Render the migration report without applying any migrations
        #[arg(short, long)]
        dry_run: bool,
        /// Skip validating the migration set against the history table
        #[arg(long)]
        skip_validate: bool,
        /// Apply unapplied migrations up through this version
        #[arg(long)]
        target_version: Option<i64>,
    },
    /// Insert migrations into the history table without applying them
    SoftApply {
        /// Render the migration report without soft applying any migrations
        #[arg(short, long)]
        dry_run: bool,
        /// Skip validating the migration set against the history table
        #[arg(long)]
        skip_validate: bool,
        /// Soft apply unapplied migrations up through this version
        #[arg(long)]
        target_version: Option<i64>,
    },
    /// List previously applied migrations
    ListApplied,
}

#[derive(Debug, Parser)]
pub enum HistoryOpts {
    /// Create the schema history table
    Init,
    /// Drop the schema history table
    Drop,
}
