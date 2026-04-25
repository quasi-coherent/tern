use tern_core::context::MigrationContext;
use tern_core::error::TernError;
use tern_core::migration::iter::DownMigrationSet;
use tern_core::migration::{DownMigration, Migration};
use tern_core::ops::MigrationOp;
use tern_core::ops::migration::Down;

use crate::ops::result::{CollectOp, OpResult};

/// Arguments for the `Revert` operation.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "cli", derive(clap::Args))]
pub struct RevertArgs {
    /// Revert migrations through this version.
    ///
    /// Runs each associated down migration query through this version, then
    /// removes the version from history.
    #[cfg_attr(feature = "cli", arg(short = 'T', long))]
    to: i64,
    /// Start the revert operation at the statement with this index.
    ///
    /// When resuming the revert operation from a previous failed attempt, this
    /// can be used this to skip to the part that failed.
    #[cfg_attr(feature = "cli", arg(long))]
    start_idx: Option<u32>,
    /// Do a "soft revert" operation.
    ///
    /// This will delete the historical record for a migration, as if the down
    /// migration ran, but without actually running the down migration query.
    #[cfg_attr(feature = "cli", arg(long))]
    soft_revert: bool,
    /// Return the migrations that the operation would apply to.
    #[cfg_attr(feature = "cli", arg(long))]
    dryrun: bool,
}

impl RevertArgs {
    /// Returns default options for the `Revert` operation.
    pub fn new(to: i64) -> Self {
        Self { to, start_idx: None, soft_revert: false, dryrun: false }
    }

    /// Start the revert operation at the statement with this index.
    pub fn start_idx(self, idx: u32) -> Self {
        Self { start_idx: Some(idx), ..self }
    }

    /// Do a "soft revert" operation.
    pub fn soft_revert(self) -> Self {
        Self { soft_revert: true, ..self }
    }

    /// Do a dry run of the operation.
    pub fn dryrun(self) -> Self {
        Self { dryrun: true, ..self }
    }

    /// Target version.
    pub fn get_to(&self) -> i64 {
        self.to
    }

    /// Get the statement index to start from.
    pub fn get_start_idx(&self) -> Option<u32> {
        self.start_idx
    }

    /// Whether this is configured to be a soft revert.
    pub fn get_soft_revert(&self) -> bool {
        self.soft_revert
    }

    /// Whether this is configured to be a dryrun.
    pub fn get_dryrun(&self) -> bool {
        self.dryrun
    }

    // The op only applies to versions v where `self.to <= v <= latest`.
    fn filter<Ctx: MigrationContext>(
        &self,
        latest: i64,
        m: &DownMigration<Ctx>,
    ) -> bool {
        let v = m.version();
        v <= latest && self.get_to() <= v
    }

    fn new_down<'a, Ctx>(&self, ctx: &'a mut Ctx) -> Down<'a, Ctx> {
        let down = if self.get_soft_revert() {
            Down::new_revert(ctx)
        } else {
            Down::new_soft_revert(ctx)
        };
        if self.get_dryrun() {
            return down.dryrun();
        }
        if let Some(idx) = self.get_start_idx() {
            down.start_idx(idx)
        } else {
            down
        }
    }
}

/// Input to the `Revert` operation.
pub struct RevertInput<Ctx> {
    set: DownMigrationSet<Ctx>,
    args: RevertArgs,
}

impl<Ctx> RevertInput<Ctx> {
    /// New `Revert` input.
    pub fn new(set: DownMigrationSet<Ctx>, args: RevertArgs) -> Self {
        Self { set, args }
    }
}

/// An operation for reverting one or more migrations.
pub struct Revert<'a, Ctx>(&'a mut Ctx);

impl<'a, Ctx: MigrationContext> Revert<'a, Ctx> {
    /// New `Revert` operation.
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self(ctx)
    }
}

impl<Ctx: MigrationContext> MigrationOp<RevertInput<Ctx>> for Revert<'_, Ctx> {
    type Output = OpResult;

    async fn exec(self, input: RevertInput<Ctx>) -> Self::Output {
        // If there have been no migrations applied, this operation doesn't make
        // sense and we return an error.
        let latest =
            self.0.latest_applied().await?.map(|l| l.version()).ok_or_else(
                || TernError::Invalid("no version to revert".into()),
            )?;
        let args = input.args;
        let iter = input.set.into_iter().filter(|m| args.filter(latest, m));

        let mut results = CollectOp::new();

        for m in iter {
            let op = args.new_down(self.0);
            let result = op.exec(&m).await;
            results.try_push(result)?;
        }

        results.ok()
    }
}
