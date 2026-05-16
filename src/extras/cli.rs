use clap::{Args, Parser};
use tern_cli::{History, Migrate, Source, TernCli as Cli, args};
use tern_core::error::TernResult;
use tern_core::migrate::{Invertible, TernMigrate, TernOptions};

use crate::app::Tern;
use crate::ops::*;

impl<T: TernMigrate, I: Args> Tern<T, I> {
    pub async fn from_options<C>() -> TernResult<Tern<Cli<T, I>>> {
        let cli = Cli::from_options::<C>().await?;
        Ok(Tern::new(cli))
    }
}

impl<T: TernMigrate> Tern<T> {
    /// Parse operation and arguments from command line arguments.
    ///
    /// The `Revert` operation is not available to this app.
    pub fn from_migrate_cli(migrate: T) -> Tern<Cli<T>> {
        let cli = Cli::from_migrate();
        Self(cli)
    }
}

impl<T: Invertible> Tern<T> {
    /// Parse operation and arguments from command line arguments.
    pub fn from_invertible_cli(migrate: T) -> Tern<Cli<T, args::Revert>> {
        let cli = Cli::from_invertible();
        Self(cli)
    }
}

impl From<args::Ls> for List {
    fn from(v: args::Ls) -> Self {
        List::new().from(v.from).to(v.to)
    }
}

impl From<args::Diff> for Diff {
    fn from(v: args::Ls) -> Self {
        let diff = Diff::new();
        if v.render_queries {
            return diff.render_queries();
        }
        diff
    }
}

impl From<args::Apply> for Apply {
    fn from(val: args::Apply) -> Self {
        let this = Self::new();
        match val.to {
            Some(v) if v.dryrun => this.to(v).dryrun(),
            Some(v) => this.to(v),
            // val.is_none()/val.all is the same as this.to.is_none()
            _ => this,
        }
    }
}

impl From<args::SoftApply> for SoftApply {
    fn from(val: args::SoftApply) -> Self {
        let this = Self::new();
        match val.to {
            Some(v) if v.dryrun => this.to(v).dryrun(),
            Some(v) => this.to(v),
            _ => this,
        }
    }
}

impl From<args::Revert> for Revert {
    fn from(val: args::Revert) -> Self {
        let this = Self::new();
        match val.to {
            Some(v) if v.dryrun => this.to(v).dryrun(),
            Some(v) => this.to(v),
            _ => this,
        }
    }
}

impl From<args::Init> for Init {
    fn from(_: args::Init) -> Self {
        Self
    }
}

impl From<args::Drop> for Drop {
    fn from(_: args::Drop) -> Self {
        Self
    }
}
