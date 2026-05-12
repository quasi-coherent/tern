//! # tern-cli
//!
//! A CLI for `tern` migration apps.
use clap::Parser;

/// The `tern` CLI application.
pub struct TernCli {
    opts: TernOpts,
}

impl TernCli {
    /// Parse CLI arguments.
    pub fn new() -> Self {
        let opts = TernOpts::parse();
        Self { opts }
    }

    /// Return a reference to the options provided on the command line.
    pub fn get_opts(&self) -> &TernOpts {
        &self.opts
    }
}

/// Command line interface for tern migrations
#[derive(Debug, Parser)]
pub struct TernOpts {
    /// tern
    #[clap(subcommand)]
    pub commands: TernCommands,
}

/// Subcommands to `tern`
#[derive(Debug, Parser)]
pub enum TernCommands {
    /// tern history
    History(HistoryOpts),
    /// tern migrate
    Migrate(MigrateOpts),
    /// tern source
    Source(SourceOpts),
}

/// Interact with the migration history table
#[derive(Debug, Parser)]
pub struct HistoryOpts {
    /// Arguments for history commands
    #[clap(subcommand)]
    pub command: HistoryCommand,
}

/// Migrate the database from one version to another
#[derive(Debug, Parser)]
pub struct MigrateOpts {
    /// Migrate subcommand
    #[clap(subcommand)]
    pub command: MigrateCommand,
}

/// Local migration source files
#[derive(Debug, Parser)]
pub struct SourceOpts {
    /// Arguments for source commands
    #[clap(subcommand)]
    pub command: SourceCommand,
}

/// Operations on the history table
#[derive(Debug, Parser)]
pub enum HistoryCommand {
    /// Create the schema history table
    Init,
    /// Drop the schema history table
    Drop,
}

/// Operations on the database
#[derive(Debug, Parser)]
pub enum MigrateCommand {
    /// Run the apply operation for a specific range of unapplied migrations
    Apply {
        /// Return the report of migrations that would be applied
        ///
        /// This will resolve queries that are not statically defined.
        /// To avoid this, use the [`Diff`] command instead.
        ///
        /// [`Diff`]: SourceArgs::Diff
        #[arg(short, long)]
        dryrun: bool,
        /// Apply versions up to and including this one
        #[arg(short, long)]
        to: Option<i64>,
    },
    /// Run the apply operation for all unapplied migration
    ApplyAll {
        /// Return the report of migrations that would be applied
        ///
        /// This will resolve queries that are not statically defined.
        /// To avoid this, use the [`Diff`] command instead.
        ///
        /// [`Diff`]: SourceArgs::Diff
        #[arg(short, long)]
        dryrun: bool,
    },
    /// Soft apply migrations
    ///
    /// This saves the specified migrations in the history table as if they had
    /// been applied without actually being applied.
    ///
    /// This can be used to sync the history table and database if starting from
    /// an existing state.
    SoftApply {
        /// Return the report of migrations that would be soft applied
        #[arg(short, long)]
        dryrun: bool,
        /// Soft apply versions up to and including this one
        #[arg(short, long)]
        to: Option<i64>,
    },
    /// Revert existing migrations
    ///
    /// For migration source that contains up and down migration pairs, this runs
    /// the down migrations to the specified version and points history to this
    /// new latest version.
    Revert {
        /// Return the report of migrations that would be revered
        #[arg(short, long)]
        dryrun: bool,
        /// Revert versions down to and including this one
        #[arg(short, long)]
        to: i64,
    }
}

/// Operations on the migration source
#[derive(Debug, Parser)]
pub enum SourceCommand {
    /// List applied migrations
    Ls {
        /// List migration versions starting from this one
        #[arg(short, long)]
        from: Option<i64>,
        /// List migration versions ending with this one
        #[arg(short, long)]
        to: Option<i64>,
    },
    /// List unapplied migrations
    Diff {
        /// Resolve migration queries in the result
        #[arg(long)]
        resolve_queries: bool,
    }
}
