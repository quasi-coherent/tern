use clap::{Args, Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Tern {
    #[clap(subcommand)]
    pub commands: TernCommands,
}

#[derive(Debug, Parser)]
pub enum TernCommands {
    Migrate(Migrate),
    History(History),
}

/// Operations on the set of migration files.
#[derive(Debug, Parser)]
pub struct Migrate {
    #[clap(subcommand)]
    pub commands: MigrateCommands,
}

/// Operations on the table storing the history of these migrations.
#[derive(Debug, Parser)]
pub struct History {
    #[clap(subcommand)]
    pub commands: HistoryCommands,
}

#[derive(Debug, Parser)]
pub enum MigrateCommands {
    /// Run any available unapplied migrations.
    ApplyAll {
        /// List the migrations to be applied without applying them.
        #[arg(short, long)]
        dryrun: bool,
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
    /// List previously applied migrations.
    ListApplied {
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
    /// Create a new migration with an auto-selected version and the given
    /// description.
    New {
        description: String,
        /// If `true`, the annotation for not running the migration in a
        /// transaction will be added to the generated file.
        no_tx: bool,
        /// Whether to create a SQL or Rust migration.
        #[arg(long = "type", value_enum)]
        migration_type: MigrationType,
        #[clap(flatten)]
        source: Source,
    },
}

#[derive(Debug, Parser)]
pub enum HistoryCommands {
    /// Create the schema history table.
    Init {
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
    /// Drop the schema history table.
    Drop {
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
    /// Do a "soft" apply of all migrations in the specified range.
    /// A soft apply will add the migration to the schema history table but
    /// without running the query for the migration.
    SoftApply {
        /// The version to start the soft apply with.
        /// If not provided, the first migration is where the soft apply starts.
        #[arg(long)]
        from_version: Option<i64>,
        /// The version to end the soft apply with.
        /// If not provided, the last migration is where the soft apply ends.
        #[arg(long)]
        to_version: Option<i64>,
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum MigrationType {
    Sql,
    Rust,
}

#[derive(Debug, Args)]
pub struct Source {
    /// Path to the folder containing migrations.
    #[clap(long)]
    pub path: PathBuf,
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
                "the `--database-url` option or the `DATABASE_URL` environment variable must be provided"
            )
        )
    }
}