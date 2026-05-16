use chrono::Utc;
use display_json::{DebugAsJson, DisplayAsJsonPretty};
use serde::Serialize;
use tern_core::error::TernError;
use tern_core::migrate::ops::{Down, Resolve, Up};
use tern_core::migrate::{Invertible, TernMigrate, TernMigrateOp};
use tern_core::migration::{Applied, Migration};

use crate::report::{
    Completed, Incomplete, MigrateOk, OpResult, TryOpResult as _,
};

/// Apply migrations in the given range if valid.
#[derive(Clone, DebugAsJson, Default, DisplayAsJsonPretty, Serialize)]
pub struct Apply {
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<i64>,
    dryrun: bool,
}

impl Apply {
    /// Returns default options for the `Apply` operation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply versions up to and including this one.
    pub fn to(self, v: i64) -> Self {
        Self { to: Some(v), ..self }
    }

    /// Return the report of the migrations that would be applied.
    ///
    /// Note that this will resolve queries that are not statically defined.
    /// To avoid this, use the command [`Diff`] instead.
    ///
    /// [`Diff`]: crate::operations::Diff
    pub fn dryrun(self) -> Self {
        Self { dryrun: true, ..self }
    }
}

impl<T: TernMigrate> TernMigrateOp<T> for Apply {
    type Success = Completed;
    type Error = Incomplete;

    async fn exec(&self, migrate: &mut T) -> OpResult<Self::Success> {
        migrate.check_history_exists().await?;
        let latest = migrate.last_applied_id().await?.map(|v| v.version());

        if let Some(v) = self.to
            && latest.is_some_and(|l| v < l)
        {
            return Err(TernError::Invalid(format!(
                "Target version {v} already applied"
            )))?;
        }

        let set = migrate.up_migrations().iter_between(latest, self.to);

        let mut oks = Vec::new();

        for m in set {
            async {
                let start = Utc::now();
                let id = m.migration_id();

                let ok = if self.dryrun {
                    let q = Resolve::new(&m).exec(migrate).await?;
                    MigrateOk::dryrun(id, Some(q), start)
                } else {
                    let applied = Up::new(&m).exec(migrate).await?;
                    MigrateOk::applied(applied)
                };
                oks.push(ok);
                Ok(())
            }
            .await
            .incomplete(&oks)?;
        }

        Ok(oks.into_iter().collect())
    }
}

/// Soft apply migrations in the given range if valid.
///
/// This means that they will be saved in the history table as if they had been
/// applied without actually being applied, which can be used to sync the
/// history table and database if history needs to start from existing state.
///
/// Valid ranges exclude migrations that already have been applied.  By default
/// if `to` is not provided, all available unapplied migrations are soft
/// applied.
#[derive(Clone, DebugAsJson, Default, DisplayAsJsonPretty, Serialize)]
pub struct SoftApply {
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<i64>,
    dryrun: bool,
}

impl SoftApply {
    /// Returns default options for the `SoftApply` operation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Soft apply up through this version.
    pub fn to(self, v: i64) -> Self {
        Self { to: Some(v), ..self }
    }

    /// Return the report of the migrations that would be soft applied.
    pub fn dryrun(self) -> Self {
        Self { dryrun: true, ..self }
    }
}

impl<T: TernMigrate> TernMigrateOp<T> for SoftApply {
    type Success = Completed;
    type Error = Incomplete;

    async fn exec(&self, migrate: &mut T) -> OpResult<Self::Success> {
        migrate.check_history_exists().await?;
        let latest = migrate.last_applied_id().await?.map(|id| id.version());

        if let Some(t) = self.to
            && latest.is_some_and(|l| t <= l)
        {
            return Err(TernError::Invalid(format!(
                "Target version {t} already exists!"
            )))?;
        }
        let set = migrate.up_migrations().iter_between(latest, self.to);

        let mut oks = Vec::new();

        for m in set {
            async {
                let id = m.migration_id();
                let applied =
                    Applied::new(&id, "-- Skipped".into(), Utc::now());
                if !self.dryrun {
                    migrate.update_applied(&applied).await?;
                }
                let ok = MigrateOk::soft_applied(applied);
                oks.push(ok);
                Ok(())
            }
            .await
            .incomplete(&oks)?;
        }

        Ok(oks.into_iter().collect())
    }
}

/// Revert migrations to the selected version.
#[derive(Clone, Default, DebugAsJson, DisplayAsJsonPretty, Serialize)]
pub struct Revert {
    to: Option<i64>,
    dryrun: bool,
}

impl Revert {
    /// Returns the `Revert` operation with target version `to`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Revert the database to the specified version.
    pub fn to(self, v: i64) -> Self {
        Self { to: Some(v), ..self }
    }

    /// Return the report of the migrations that would be reverted.
    pub fn dryrun(self) -> Self {
        Self { dryrun: true, ..self }
    }
}

impl<T: Invertible> TernMigrateOp<T> for Revert {
    type Success = Completed;
    type Error = Incomplete;

    async fn exec(&self, migrate: &mut T) -> OpResult<Self::Success> {
        migrate.check_history_exists().await?;
        let latest = migrate.last_applied_id().await?.map(|id| id.version());

        if let Some(t) = self.to
            && latest.is_some_and(|l| l > t)
        {
            return Err(TernError::Invalid(format!(
                "Target version {t} does not exist!"
            )))?;
        }
        let set = migrate.down_migrations().iter_between(latest, self.to);

        let mut oks = Vec::new();

        for m in set {
            async {
                let start = Utc::now();
                let id = m.migration_id();

                let ok = if self.dryrun {
                    let q = Resolve::new(&m).exec(migrate).await?;
                    MigrateOk::dryrun(id, Some(q), start)
                } else {
                    let applied = Down::new(&m).exec(migrate).await?;
                    MigrateOk::reverted(applied)
                };
                oks.push(ok);
                Ok(())
            }
            .await
            .incomplete(&oks)?;
        }

        Ok(oks.into_iter().collect())
    }
}
