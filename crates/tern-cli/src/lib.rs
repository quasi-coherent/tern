//! # tern-cli
//!
//! A CLI for `tern` migration apps.
use clap::{Args, Parser, Subcommand};
use std::ops::{Deref, DerefMut};
use tern_core::error::TernResult;
use tern_core::migrate::{Invertible, TernMigrate, TernOptions};

mod history;
pub use history::History;

mod migrate;
pub use migrate::Migrate;

mod source;
pub use source::Source;

/// Arguments for `tern` CLI subcommands.
pub mod args {
    pub use super::history::{Drop, Init};
    pub use super::migrate::{Apply, Revert, SoftApply};
    pub use super::source::{Diff, Ls};
}

/// Disabled
#[derive(Clone, Copy, Debug, Default, Args)]
pub struct Disabled;

#[derive(Parser)]
pub struct TernCommand<I: Args, C: Args> {
    #[command(subcommand)]
    commands: TernCommands<I>,
    #[clap(flatten)]
    options: C,
}

/// `tern`
#[derive(Debug, Subcommand)]
#[non_exhaustive]
pub enum TernCommands<I: Args> {
    /// `tern history`
    History(History),
    /// `tern source`
    Source(Source),
    /// `tern migrate`
    Migrate(Migrate<I>),
}

/// Cli for a `tern` migration app.
pub struct TernCli<T, I = Disabled>
where
    I: Args,
{
    migrate: T,
    commands: TernCommands<I>,
}

impl<T: TernMigrate, I: Args> TernCli<T, I> {
    /// Initialize completely from command-line arguments.
    pub async fn from_options<C>() -> TernResult<TernCli<T, I>>
    where
        C: TernOptions<T> + Args,
    {
        let cli = TernCommand::<I, C>::parse();
        let migrate = cli.options.connect().await?;
        Ok(TernCli { migrate, commands: cli.commands })
    }

    /// Return a reference to the CLI arguments.
    pub fn commands(&self) -> &TernCommands<I> {
        &self.commands
    }

    /// Consume this type and return the inner `T`.
    pub fn into_inner(self) -> T {
        self.migrate
    }
}

impl<T: TernMigrate> TernCli<T> {
    /// Initialize with migrations `T`.
    pub fn from_migrate(migrate: T) -> TernCli<T> {
        let cli = TernCommand::<Disabled, Disabled>::parse();
        TernCli { migrate, commands: cli.commands }
    }
}

impl<T: Invertible> TernCli<T, args::Revert> {
    /// Initialize with reversible migrations `T`.
    pub fn from_invertible(migrate: T) -> TernCli<T, args::Revert> {
        let cli = TernCommand::<args::Revert, Disabled>::parse();
        TernCli { migrate, commands: cli.commands }
    }
}

impl<T, I: Args> Deref for TernCli<T, I> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.migrate
    }
}

impl<T, I: Args> DerefMut for TernCli<T, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.migrate
    }
}
