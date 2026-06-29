use chrono::Utc;
use tern_core::context::MigrationContext;
use tern_core::error::{TernError, TernResult};
use tern_core::migration::iter::UpMigrationSet;
use tern_core::migration::{Migration, MigrationData, UpMigration};
use tern_core::ops::MigrationOp;
use tern_core::ops::migration::{RenderQuery, StaticQuery};

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

    fn find_version<Ctx: MigrationContext>(
        &self,
        m: &UpMigration<Ctx>,
    ) -> bool {
        self.version == m.version()
    }
}

/// Input to the `Show` operation.
pub struct ShowInput<Ctx> {
    set: UpMigrationSet<Ctx>,
    args: ShowArgs,
}

impl<Ctx> ShowInput<Ctx> {
    /// New `ShowInput`.
    pub fn new(set: UpMigrationSet<Ctx>, args: ShowArgs) -> Self {
        Self { set, args }
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

impl<Ctx: MigrationContext> MigrationOp<ShowInput<Ctx>> for Show<'_, Ctx> {
    type Output = TernResult<MigrationData>;

    async fn exec(self, input: ShowInput<Ctx>) -> Self::Output {
        let args = input.args;
        let version = args.get_version();

        let Some(m) = input.set.into_iter().find(|m| !args.find_version(m))
        else {
            return Err(TernError::Invalid(format!(
                "migration V{version} not found"
            )))?;
        };

        let id = m.migration_id();
        let start = Utc::now();

        if args.get_render_query() {
            RenderQuery::new(self.0).exec(&m).await
        } else {
            StaticQuery::new().exec(&m).await
        }
        .map(|q| MigrationData::new(id, &q, start))
    }
}
