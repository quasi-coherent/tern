//! CLI functionality for tern applications.
//!
//! The module exposes [`TernCli`], a [`clap`] application that packages tern
//! operations and their arguments into subcommands and subcommand options of
//! a CLI.
//!
//! [`clap`]: https://docs.rs/clap/latest/clap/
use clap::builder::styling::{AnsiColor, Styles};
use clap::{Args, Parser};
use tern_cli::ConnOpt;
use tern_core::error::TernResult;
use tern_core::migration::Migration;

use crate::app::{Tern, TernApp};
use crate::ops::{self, OpComplete, OpResult};

#[doc(hidden)]
pub extern crate clap;

// _Not_ CARGO_PKG_NAME, which is _this_ crate.
//
// Unfortunately there isn't a way to get author, version, etc. for the
// other things that a clap parser can display in help menus and so on...
// At least as far as I can tell.  There are proc macro crates in varying forms
// of "abandoned toy project" but we can't use proc macros because where we're
// putting this data is from a proc macro.
const USER_CRATE: &str = env!("CARGO_CRATE_NAME");

const HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Blue.on_default().bold())
    .usage(AnsiColor::Blue.on_default().bold())
    .literal(AnsiColor::White.on_default())
    .placeholder(AnsiColor::Green.on_default());

/// A `TernApp` in CLI form.
pub struct TernCli<T>(Tern<T>, Cmd);

impl<T> TernCli<T>
where
    T: TernApp + 'static,
    <T::Set as Iterator>::Item: Migration<Ctx = T> + 'static,
{
    /// Create the CLI to a `TernApp`.
    ///
    /// This requires that [`TernApp::Options`] be translated to CLI options via
    /// the [`clap::Args`] interface for this `T`.
    pub async fn try_init() -> TernResult<TernCli<T>>
    where
        T::Options: Args,
    {
        let Opt { command, conn, options } = Opt::<T::Options>::parse();
        let conn = conn.get_db_url()?;
        let app = Tern::init(&conn, options).await?;
        Ok(TernCli(app, command))
    }

    /// Run the command parsed from CLI options with the given tern app.
    pub async fn run(mut self) -> OpResult {
        match self.1 {
            Cmd::History(hist) => hist.run_history(&mut self.0).await,
            Cmd::Migrate(mig) => mig.run_migrate(&mut self.0).await,
            Cmd::Source(src) => src.run_source(&mut self.0).await,
        }
    }
}

#[derive(Debug, Parser)]
#[clap(about = USER_CRATE, styles = HELP_STYLES)]
struct Opt<U: Args = ()> {
    #[clap(subcommand)]
    command: Cmd,
    #[clap(flatten)]
    conn: ConnOpt,
    #[clap(flatten)]
    options: U,
}

/// Group of commands for operations related to migrating database versions.
#[derive(Clone, Copy, Debug, Parser)]
enum Cmd {
    #[clap(alias = "mig")]
    Migrate(MigrateOpt),
    #[clap(alias = "hist")]
    History(HistoryOpt),
    #[clap(alias = "src")]
    Source(SourceOpt),
}

/// Commands for applying or reverting migrations.
#[derive(Clone, Copy, Debug, Parser)]
pub struct MigrateOpt {
    #[clap(subcommand)]
    command: MigrateCommand,
}

impl MigrateOpt {
    async fn run_migrate<T>(self, tern: &mut Tern<T>) -> OpResult
    where
        T: TernApp + 'static,
        <T::Set as Iterator>::Item: Migration<Ctx = T> + 'static,
    {
        match self.command {
            MigrateCommand::Apply(args) => tern.apply(args).await,
            MigrateCommand::Revert(args) => tern.revert(args).await,
        }
    }
}

/// Subcommands of the `migrate` command.
#[derive(Clone, Copy, Debug, Parser)]
#[non_exhaustive]
pub enum MigrateCommand {
    /// Migrate the database to a new version by applying migrations.
    Apply(ops::migrate::ApplyArgs),
    /// Migrate the database to a previous version by reverting migrations.
    #[clap(alias = "undo")]
    Revert(ops::migrate::RevertArgs),
}

/// Commands for creating or dropping the table storing migration history.
#[derive(Clone, Copy, Debug, Parser)]
pub struct HistoryOpt {
    #[clap(subcommand)]
    command: HistoryCommand,
}

impl HistoryOpt {
    async fn run_history<T>(self, tern: &mut Tern<T>) -> OpResult
    where
        T: TernApp + 'static,
        <T::Set as Iterator>::Item: Migration<Ctx = T> + 'static,
    {
        match self.command {
            HistoryCommand::Create => tern.create_if_not_exists().await,
            HistoryCommand::Drop => tern.drop_if_exists().await,
        }?;
        Ok(OpComplete::default())
    }
}

/// Subcommands of the `history` subcommand.
#[derive(Clone, Copy, Debug, Parser)]
#[non_exhaustive]
pub enum HistoryCommand {
    /// Initialize a tern application's history table.
    Create,
    /// Drop a tern application's history table.
    Drop,
}

/// Commands for viewing the source of a migration set.
#[derive(Clone, Copy, Debug, Parser)]
pub struct SourceOpt {
    #[clap(subcommand)]
    command: SourceCommand,
}

impl SourceOpt {
    async fn run_source<T>(self, tern: &mut Tern<T>) -> OpResult
    where
        T: TernApp + 'static,
        <T::Set as Iterator>::Item: Migration<Ctx = T> + 'static,
    {
        match self.command {
            SourceCommand::List(args) => tern.list(args).await,
            SourceCommand::Show(args) => tern.show(args).await,
        }
    }
}

/// Subcommands of the `source` command.
#[derive(Clone, Copy, Debug, Parser)]
#[non_exhaustive]
pub enum SourceCommand {
    /// List migrations in the migration set.
    #[clap(alias = "ls")]
    List(ops::source::ListArgs),
    /// Show the contents of migrations in the migration set.
    Show(ops::source::ShowArgs),
}
