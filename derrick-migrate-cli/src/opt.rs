use clap::{Args, Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Opt {
    /// Optional config file.
    /// Not implemented yet.
    #[arg(skip = None)]
    _config: Option<PathBuf>,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    Migrate(MigrateOpt),
}

#[derive(Debug, Parser)]
pub struct MigrateOpt {
    #[clap(subcommand)]
    pub command: MigrateCommand,
}

#[derive(Debug, Parser)]
pub enum MigrateCommand {
    /// Create a new migration with an auto-selected version
    /// and the given description.
    New {
        description: String,
        /// If `true`, the annotation for not running the migration
        /// in a transaction will be added to the generated file.
        no_tx: bool,
        /// Whether to create a SQL or Rust migration.
        #[arg(long = "type", value_enum)]
        migration_type: MigrationType,
        #[clap(flatten)]
        source: Source,
    },
    /// List all applied migrations.
    List {
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
    /// Validate this migration source against the migration
    /// history.
    Validate {
        #[clap(flatten)]
        connect_opts: ConnectOpts,
    },
    /// Run any available unapplied migrations.
    Run {
        /// List the migrations to be applied without
        /// applying them.
        #[arg(short, long)]
        dry_run: bool,
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
    /// Connection string for the database either from the command
    /// line or from the environment variable `DATABASE_URL`.
    #[clap(long, short = 'D', env)]
    pub database_url: Option<String>,
    /// Optional destination table name in the destination
    /// schema.  The default is `_derrick_migrations`.
    pub history_table: Option<String>,
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
