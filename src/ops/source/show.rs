use chrono::Utc;
use tern_core::error::TernError;

use tern_core::context::MigrationContext;
use tern_core::migration::{Migration, MigrationData};
use tern_core::ops::Operation;
use tern_core::ops::migration::{RenderQuery, StaticQuery};

use crate::migration::MigrationBoxSet;
use crate::ops::result::{CollectOp, OpResult};

/// Arguments to the `Show` operation.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "cli", derive(clap::Args))]
pub struct ShowArgs {
    /// The version to show.
    #[cfg_attr(feature = "cli", arg(short = 'v', long))]
    version: i64,
    /// Whether to render the query of a dynamically defined migration.
    ///
    /// **WARNING**: This should be used with great care and caution.  It runs
    /// the actual async computation for the migration, which can in theory
    /// mean doing anything that the migration context can do.
    #[cfg_attr(feature = "cli", arg(long))]
    render_query: bool,
}

impl ShowArgs {
    /// Returns default options for the `Show` operation.
    pub fn new(version: i64) -> Self {
        Self { version, render_query: false }
    }

    /// When `diff` is enabled, resolve queries for unapplied migrations that
    /// get their query when applied.
    pub fn render_query(self) -> Self {
        Self { render_query: true, ..self }
    }

    /// The version to show.
    pub fn get_version(&self) -> i64 {
        self.version
    }

    /// Whether configured to resolve all queries in results.
    pub fn get_render_query(&self) -> bool {
        self.render_query
    }
}

/// Input to the `Show` operation.
pub struct ShowInput<Ctx> {
    set: MigrationBoxSet<Ctx>,
    args: ShowArgs,
}

impl<Ctx: MigrationContext> ShowInput<Ctx> {
    /// New `ShowInput`.
    pub fn new<I>(iter: I, args: ShowArgs) -> Self
    where
        I: Iterator<Item: Migration<Ctx = Ctx> + 'static>,
    {
        Self { set: iter.collect(), args }
    }
}

/// An operation for showing the migrations in the current migration set.
#[derive(Debug)]
pub struct Show<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> Show<'a, Ctx> {
    /// New `Show` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> Operation<ShowInput<Ctx>> for Show<'_, Ctx> {
    type Output = OpResult;

    async fn exec(self, input: ShowInput<Ctx>) -> Self::Output {
        let start = Utc::now();
        let ShowInput { set, args } = input;

        let mut ms =
            set.into_iter().skip_while(|m| m.version() != args.version);
        let Some(migration) = ms.next() else {
            return Err(TernError::Invalid(format!(
                "missing version {}",
                args.version
            )))?;
        };

        let v = migration.version();
        let descr = migration.description();
        let mut results = CollectOp::new();

        let res = if args.get_render_query() {
            RenderQuery::new(self.0).exec(&migration).await
        } else {
            Ok(StaticQuery::new().exec(&migration).await)
        }
        .map(|q| {
            let mut data = MigrationData::new(v, descr, &q);
            data.finished(start);
            data
        });
        results.try_push(res)?;

        results.ok()
    }
}
