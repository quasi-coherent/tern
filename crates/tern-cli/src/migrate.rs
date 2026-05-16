use clap::{Args, Subcommand};

/// `tern migrate`
#[derive(Args, Debug)]
pub struct Migrate<I: Args> {
    #[command(subcommand)]
    command: MigrateCommands<I>,
}

/// Operations on the database with a set of migrations
#[derive(Debug, Subcommand)]
pub enum MigrateCommands<I: Args> {
    Apply(Apply),
    SoftApply(SoftApply),
    Revert(I),
}
/// Apply migrations to create a new version of the database
#[derive(Args, Debug)]
pub struct Apply {
    /// Prepare and return the migrations that would be applied in the operation
    #[arg(long)]
    pub dryrun: bool,
    /// Apply available migrations through this version
    #[arg(short, long, group = "apply")]
    pub to: Option<i64>,
    /// Apply all available migrations
    #[arg(long, group = "apply", conflicts_with = "to")]
    pub all: bool,
}

/// Apply migrations to only the history table
#[derive(Args, Debug)]
pub struct SoftApply {
    /// Return the migrations that would be soft applied
    #[arg(long)]
    pub dryrun: bool,
    /// Soft apply migrations through this version
    #[arg(short, long, group = "apply")]
    pub to: Option<i64>,
    /// Soft apply all available migrations
    #[arg(long, group = "apply", conflicts_with = "to")]
    pub all: bool,
}

/// Revert to a previous version of the database
#[derive(Args, Debug)]
pub struct Revert {
    /// Return the migrations that would be soft applied
    #[arg(long)]
    pub dryrun: bool,
    /// Soft apply migrations through this version
    #[arg(short, long)]
    pub to: i64,
}
