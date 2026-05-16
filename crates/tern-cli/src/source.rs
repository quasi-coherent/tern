use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct Source {
    #[command(subcommand)]
    commands: SourceCommands,
}

/// Operations on the source of migrations
#[derive(Debug, Subcommand)]
pub enum SourceCommands {
    Diff(Diff),
    Ls(Ls),
}

/// List applied migrations
#[derive(Args, Clone, Copy, Debug)]
pub struct Ls {
    /// Earliest version to return in the results
    #[arg(short, long)]
    pub from: Option<i64>,
    /// Latest version to return in the results
    #[arg(short, long)]
    pub to: Option<i64>,
}

/// Compute the difference between local source and remote history
#[derive(Args, Clone, Copy, Debug)]
pub struct Diff {
    /// Resolve the full migration query content for results
    #[arg(long)]
    pub render_queries: bool,
}
