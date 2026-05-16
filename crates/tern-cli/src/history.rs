use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct History {
    #[command(subcommand)]
    command: HistoryCommands,
}

/// Operations on the migration history table
#[derive(Debug, Subcommand)]
pub enum HistoryCommands {
    Drop(Drop),
    Init(Init),
}

/// Create the history table
#[derive(Args, Debug)]
pub struct Init;

/// Drop the history table
#[derive(Args, Debug)]
pub struct Drop;
