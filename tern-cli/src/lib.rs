use clap::{Args, Parser};

/// Command-line interface for running a `tern` migration application
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct CliOpts {
    #[clap(subcommand)]
    pub opts: Opts,
}

impl Default for CliOpts {
    fn default() -> Self {
        Self::new()
    }
}

impl CliOpts {
    /// Create a new `CliOpts` by parsing command line options.
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
        dryrun: bool,
        /// Apply unapplied migrations up through this version
        #[arg(long)]
        target_version: Option<i64>,
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
    /// Insert migrations into the history table without applying them
    SoftApply {
        /// Render the migration report without soft applying any migrations
        #[arg(short, long)]
        dryrun: bool,
        /// Soft apply unapplied migrations up through this version
        #[arg(long)]
        target_version: Option<i64>,
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
    /// List previously applied migrations
    ListApplied {
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
}

#[derive(Debug, Parser)]
pub enum HistoryOpts {
    /// Create the schema history table
    Init {
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
    /// Drop the schema history table
    Drop {
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
}

#[derive(Debug, Args)]
pub struct ConnectOpts {
    /// Connection string for the database either from the command line or from
    /// the environment variable `DATABASE_URL`.
    #[clap(long, short = 'D', env)]
    pub database_url: Option<String>,
}

impl ConnectOpts {
    pub fn required_db_url(&self) -> anyhow::Result<&str> {
        self.database_url.as_deref().ok_or_else(
            || anyhow::anyhow!(
                "the `--database-url/-D` option or the `DATABASE_URL` environment variable must be provided"
            )
        )
    }
}
