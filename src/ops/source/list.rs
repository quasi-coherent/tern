use chrono::Utc;
use tern_core::context::MigrationContext;
use tern_core::migration::iter::UpMigrationSet;
use tern_core::migration::{Migration, MigrationData, UpMigration};
use tern_core::ops::MigrationOp;
use tern_core::ops::migration::StaticQuery;

use crate::ops::result::{CollectOp, OpResult};

/// Arguments to the `List` operation.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "cli", derive(clap::Args))]
pub struct ListArgs {
    /// Return results starting with this version.
    #[cfg_attr(feature = "cli", arg(short = 'F', long, group = "list"))]
    from: Option<i64>,
    /// Return results through this version.
    #[cfg_attr(feature = "cli", arg(short = 'T', long, group = "list"))]
    to: Option<i64>,
    /// Show unapplied migrations only.
    #[cfg_attr(feature = "cli",arg(long, group = "list", conflicts_with_all = ["from", "to"]))]
    diff: bool,
}

impl ListArgs {
    /// Returns default options for the `List` operation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Limit results to versions starting with this one.
    pub fn from(self, v: i64) -> Self {
        Self { from: Some(v), ..self }
    }

    /// Limit results to versions ending with this one.
    pub fn to(self, v: i64) -> Self {
        Self { to: Some(v), ..self }
    }

    /// Only return unapplied migrations.
    pub fn diff(self) -> Self {
        Self { diff: true, ..self }
    }

    /// The configured start version of the results.
    pub fn get_from(&self) -> Option<i64> {
        self.from
    }

    /// The configured end version of the results.
    pub fn get_to(&self) -> Option<i64> {
        self.to
    }

    /// Whether configured to return just the unapplied migrations.
    pub fn get_diff(&self) -> bool {
        self.diff
    }

    fn in_range<Ctx: MigrationContext>(
        &self,
        latest: Option<i64>,
        m: &UpMigration<Ctx>,
    ) -> bool {
        let version = m.version();
        if self.get_diff() {
            latest.is_none_or(|l| l < version)
        } else {
            self.get_from().is_none_or(|f| f <= version)
                && self.get_to().is_none_or(|t| version <= t)
        }
    }
}

/// Input to the `List` operation.
pub struct ListInput<Ctx> {
    set: UpMigrationSet<Ctx>,
    args: ListArgs,
}

impl<Ctx> ListInput<Ctx> {
    /// New `ListInput`.
    pub fn new(set: UpMigrationSet<Ctx>, args: ListArgs) -> Self {
        Self { set, args }
    }
}

/// An operation for showing the migrations in the current migration set.
#[derive(Debug)]
pub struct List<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> List<'a, Ctx> {
    /// New `List` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> MigrationOp<ListInput<Ctx>> for List<'_, Ctx> {
    type Output = OpResult;

    async fn exec(self, input: ListInput<Ctx>) -> Self::Output {
        let latest = self.0.latest_applied().await?.map(|data| data.version());
        let args = input.args;
        let iter = input.set.into_iter().filter(|m| args.in_range(latest, m));

        let mut results = CollectOp::new();

        for m in iter {
            let id = m.migration_id();
            let start = Utc::now();
            let result = StaticQuery::new()
                .exec(&m)
                .await
                .map(|q| MigrationData::new(id, &q, start));

            results.try_push(result)?;
        }

        results.ok()
    }
}
