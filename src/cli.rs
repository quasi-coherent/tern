use clap::builder::styling::{AnsiColor, Styles};
use clap::{Args, Parser};
use futures_core::future::Future;
use tern_core::error::TernResult;

use crate::app::{ContextOptions, Tern, TernApp};
use crate::ops::{self, OpComplete, OpResult};

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

impl<T: TernApp> Tern<T> {
    /// Run the `tern` command from CLI options.
    pub async fn run(self) -> OpResult {
        let Opt { command, .. } = Opt::parse_opt();
        self.run_command(command).await
    }

    /// Run the `tern` command from CLI options with app "factory" `F`.
    pub async fn run_new<F, Fut>(mut f: F) -> OpResult
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = TernResult<T>>,
    {
        let Opt { command, .. } = Opt::parse_opt();
        let app = f().await?;
        Tern::new(app).run_command(command).await
    }

    /// Run the `tern` command from an async function of the CLI options.
    pub async fn run_with<U, F, Fut>(mut f: F) -> OpResult
    where
        U: Args,
        F: FnMut(U) -> Fut,
        Fut: Future<Output = TernResult<T>>,
    {
        let Opt { command, options } = Opt::<U>::parse();
        let app = f(options).await?;
        Tern::new(app).run_command(command).await
    }

    /// Run the `tern` command from CLI options having a `U: ContextOptions<T>`.
    pub async fn run_options<U>() -> OpResult
    where
        U: ContextOptions<T> + Args,
    {
        Tern::run_with(|opts: U| opts.initialize()).await
    }

    async fn run_command(mut self, cmd: Cmd) -> OpResult {
        match cmd {
            Cmd::History(hist) => hist.run_history(&mut self).await,
            Cmd::Migrate(mig) => mig.run_migrate(&mut self).await,
            Cmd::Source(src) => src.run_source(&mut self).await,
        }
    }
}

#[derive(Debug, Parser)]
#[clap(about = USER_CRATE, styles = HELP_STYLES)]
struct Opt<U: Args = ()> {
    #[clap(subcommand)]
    command: Cmd,
    #[clap(flatten)]
    options: U,
}

impl Opt {
    fn parse_opt() -> Self {
        Self::parse()
    }
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
    pub command: MigrateCommand,
}

impl MigrateOpt {
    async fn run_migrate<T: TernApp>(self, tern: &mut Tern<T>) -> OpResult {
        match self.command {
            MigrateCommand::Apply(args) => tern.apply(args).await,
            MigrateCommand::Revert(args) => tern.revert(args).await,
        }
    }
}

#[derive(Clone, Copy, Debug, Parser)]
#[non_exhaustive]
pub enum MigrateCommand {
    Apply(ops::migrate::ApplyArgs),
    #[clap(alias = "undo")]
    Revert(ops::migrate::RevertArgs),
}

/// Commands for creating or dropping the table storing migration history.
#[derive(Clone, Copy, Debug, Parser)]
pub struct HistoryOpt {
    #[clap(subcommand)]
    pub command: HistoryCommand,
}

impl HistoryOpt {
    async fn run_history<T: TernApp>(self, tern: &mut Tern<T>) -> OpResult {
        match self.command {
            HistoryCommand::Init => tern.init().await,
            HistoryCommand::Drop => tern.drop_if_exists().await,
        }?;
        Ok(OpComplete::default())
    }
}

#[derive(Clone, Copy, Debug, Parser)]
#[non_exhaustive]
pub enum HistoryCommand {
    Init,
    Drop,
}

/// Commands for viewing the source of a migration set.
#[derive(Clone, Copy, Debug, Parser)]
pub struct SourceOpt {
    #[clap(subcommand)]
    pub command: SourceCommand,
}

impl SourceOpt {
    async fn run_source<T: TernApp>(self, tern: &mut Tern<T>) -> OpResult {
        match self.command {
            SourceCommand::List(args) => tern.list(args).await,
            SourceCommand::Show(args) => tern.show(args).await,
        }
    }
}

#[derive(Clone, Copy, Debug, Parser)]
#[non_exhaustive]
pub enum SourceCommand {
    #[clap(alias = "ls")]
    List(ops::source::ListArgs),
    Show(ops::source::ShowArgs),
}
