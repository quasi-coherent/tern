use tern_core::context::{MigrationContext, MigrationExecutor as _};
use tern_core::error::TernError;
use tern_core::migration::Migration;
use tern_core::migration::iter::MigrationSetExt as _;
use tern_core::ops::Operation;
use tern_core::ops::migration::RevertOne;

use crate::migration::{MigrationBox, MigrationBoxSet};
use crate::ops::result::{CollectOp, OpResult};

/// Arguments for the `Revert` operation.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "cli", derive(clap::Args))]
pub struct RevertArgs {
    /// Revert migrations through this version.
    ///
    /// Runs each associated down migration query through this version, then
    /// removes the version from history.
    #[cfg_attr(feature = "cli", arg(short = 'T', long))]
    to: i64,
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
    pub fn new() -> Self {
        Self::default()
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

    /// Whether this is configured to be a soft revert.
    pub fn get_soft_revert(&self) -> bool {
        self.soft_revert
    }

    /// Whether this is configured to be a dryrun.
    pub fn get_dryrun(&self) -> bool {
        self.dryrun
    }

    fn filter<Ctx: MigrationContext>(
        &self,
        latest: i64,
        m: &MigrationBox<Ctx>,
    ) -> bool {
        m.version() <= latest && m.version() >= self.to
    }

    fn new_revert_one<'a, Ctx>(&self, ctx: &'a mut Ctx) -> RevertOne<'a, Ctx> {
        let revert = if self.get_soft_revert() {
            RevertOne::new(ctx).soft_revert()
        } else {
            RevertOne::new(ctx)
        };
        if self.get_dryrun() {
            return revert.dryrun();
        }
        revert
    }
}

/// Input to the `Revert` operation.
pub struct RevertInput<Ctx> {
    set: MigrationBoxSet<Ctx>,
    args: RevertArgs,
}

impl<Ctx: MigrationContext> RevertInput<Ctx> {
    /// New `Revert` input.
    pub fn new<I>(iter: I, args: RevertArgs) -> Self
    where
        I: DoubleEndedIterator<Item: Migration<Ctx = Ctx> + 'static>,
    {
        Self { set: iter.collect(), args }
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

impl<Ctx: MigrationContext + 'static> Operation<RevertInput<Ctx>>
    for Revert<'_, Ctx>
{
    type Output = OpResult;

    async fn exec(self, input: RevertInput<Ctx>) -> Self::Output {
        let history = self.0.history_table();
        // If there have been no migrations applied, this operation doesn't make
        // sense and we return an error.
        let latest = self
            .0
            .executor_mut()
            .current_version(history)
            .await?
            .map(|data| data.version())
            .ok_or_else(|| TernError::Invalid("no version to revert".into()))?;

        let RevertInput { set, args } = input;

        // The down migration subset selected necessarily has max version equal
        // to the latest applied.
        if set.version() != latest {
            return Err(TernError::Invalid(format!(
                "version error: last applied: {latest}, last from selection: {}",
                set.version(),
            )))?;
        }

        let mut migs = set.into_iter().range(Some(args.get_to()), None);
        let mut results = CollectOp::new();

        while let Some(migration) = migs.next_back() {
            if migration.version() >= args.to {
                let op = args.new_revert_one(self.0);
                let result = op.exec(&migration).await;
                results.try_push(result)?;
            }
        }

        results.ok()
    }
}
