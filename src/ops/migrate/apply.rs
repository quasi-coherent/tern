use tern_core::context::MigrationContext;
use tern_core::migration::iter::UpMigrationSet;
use tern_core::migration::{Migration, UpMigration};
use tern_core::ops::MigrationOp;
use tern_core::ops::migration::Up;

use crate::ops::result::{CollectOp, OpResult};

/// Arguments for the `Apply` operation.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "cli", derive(clap::Args))]
pub struct ApplyArgs {
    /// Apply migrations up through this version.
    ///
    /// If omitted, the default is to apply the next unapplied migration.
    #[cfg_attr(feature = "cli", arg(short = 'T', long, group = "apply"))]
    to: Option<i64>,
    /// Apply all available migrations.
    #[cfg_attr(
        feature = "cli",
        arg(long, group = "apply", conflicts_with = "to")
    )]
    all: bool,
    #[cfg_attr(feature = "cli", arg(long))]
    soft_apply: bool,
    /// Return the migrations that the operation would apply to.
    #[cfg_attr(feature = "cli", arg(long))]
    dryrun: bool,
}

impl ApplyArgs {
    /// Returns default options for the `Apply` operation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply migrations through this version.
    pub fn to(self, v: i64) -> Self {
        Self { to: Some(v), ..self }
    }

    /// Apply all available migrations.
    ///
    /// # Errors
    ///
    /// Returns an error if `to` is also set.
    pub fn all(self) -> Self {
        Self { all: true, ..self }
    }

    /// Do a "soft apply" of the operation.
    ///
    /// This means that the query for the migration will be skipped, but the
    /// migration will be inserted into the history table as if it had been
    /// applied.
    ///
    /// This is used to sync the history table and existing state, for instance
    /// if starting from an existing collection of migrations.
    pub fn soft_apply(self) -> Self {
        Self { soft_apply: true, ..self }
    }

    /// Do a dry run of the operation.
    pub fn dryrun(self) -> Self {
        Self { dryrun: true, ..self }
    }

    /// Target version.
    pub fn get_to(&self) -> Option<i64> {
        self.to
    }

    /// Whether all migrations are to be applied.
    pub fn get_all(self) -> bool {
        self.all
    }

    /// Whether this is configured to be a soft apply.
    pub fn get_soft_apply(self) -> bool {
        self.soft_apply
    }

    /// Whether this is configured to be a dryrun.
    pub fn get_dryrun(self) -> bool {
        self.dryrun
    }

    // Filter on `UpMigrationSet.into_iter()` to select candidate migrations.
    //
    // For a version v, the lower bound `latest < v` is always true and
    //  `v <= self.to` should hold.
    //
    // But if `self.to.is_none()` this collapses to `v == latest + 1` by default
    // behavior.  Unless `--all` is found, in which case `v` is not bounded
    // above.
    fn filter<Ctx: MigrationContext>(
        &self,
        latest: i64,
        m: &UpMigration<Ctx>,
    ) -> bool {
        let v = m.version();
        latest < v
            && self
                .get_to()
                .map(|t| v <= t)
                .unwrap_or(self.get_all() || v == latest + 1)
    }

    // Returns the correct `Up` (with dryrun, soft apply).
    fn new_up<'a, Ctx>(&self, ctx: &'a mut Ctx) -> Up<'a, Ctx> {
        let up = if self.get_soft_apply() {
            Up::new_soft_apply(ctx)
        } else {
            Up::new_apply(ctx)
        };
        if self.get_dryrun() {
            return up.dryrun();
        }
        up
    }
}

/// Input to the `Apply` operation.
pub struct ApplyInput<Ctx> {
    set: UpMigrationSet<Ctx>,
    args: ApplyArgs,
}

impl<Ctx> ApplyInput<Ctx> {
    /// New `Apply` input.
    pub fn new(set: UpMigrationSet<Ctx>, args: ApplyArgs) -> Self {
        Self { set, args }
    }
}

/// An operation for applying one or more migrations.
pub struct Apply<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> Apply<'a, Ctx> {
    /// New `Apply` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> MigrationOp<ApplyInput<Ctx>> for Apply<'_, Ctx> {
    type Output = OpResult;

    async fn exec(self, input: ApplyInput<Ctx>) -> Self::Output {
        // If there have been no migrations applied, `latest` will come back
        // `None` here, and then we set it to 0, which is the right thing to do
        // given how we filter with `ApplyArgs::filter` above.
        let latest = self
            .0
            .latest_applied()
            .await?
            .map(|data| data.version())
            .unwrap_or_default();
        let args = input.args;
        let iter = input.set.into_iter().filter(|m| args.filter(latest, m));
        let mut results = CollectOp::new();
        for m in iter {
            let op = args.new_up(self.0);
            let result = op.exec(&m).await;
            results.try_push(result)?;
        }
        results.ok()
    }
}
