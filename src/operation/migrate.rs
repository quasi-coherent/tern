use chrono::Utc;
use display_json::{DebugAsJson, DisplayAsJsonPretty};
use serde::Serialize;
use tern_core::error::TernError;
use tern_core::migration::{Applied, Migration};

use crate::migrate::{Invertible, TernMigrate};
use crate::migration::types::*;
use crate::operation::TernMigrateOp;
use crate::report::{Completed, MigrateOk, Report};

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
    type Output = Completed;

    async fn exec(&self, migrate: &mut T) -> Report<Self::Output> {
        migrate.check_history_exists().await?;
        let latest = migrate.last_applied_id().await?.map(|v| v.version());

        if let Some(v) = self.to
            && latest.is_some_and(|l| v < l)
        {
            return Err(TernError::Invalid(format!(
                "Target version {v} already applied"
            )))?;
        }
        let set = migrate.up_migrations().into_iter().range(latest, self.to);

        let mut oks = Vec::new();

        for m in set {
            async {
                let ok = if self.dryrun {
                    let id = m.migration_id();
                    let query = m.query(migrate).await?;
                    MigrateOk::dryrun(id, Some(query))
                } else {
                    let applied = m.apply(migrate).await?;
                    m.post_apply(migrate, &applied).await?;
                    MigrateOk::applied(applied)
                };
                oks.push(ok);
                Ok(())
            }
            .await
            .report(&oks)?;
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
    type Output = Completed;

    async fn exec(&self, migrate: &mut T) -> Report<Self::Output> {
        migrate.check_history_exists().await?;
        let latest = migrate.last_applied_id().await?.map(|id| id.version());

        if let Some(t) = self.to
            && latest.is_some_and(|l| t <= l)
        {
            return Err(TernError::Invalid(format!(
                "Target version {t} already exists!"
            )))?;
        }
        let set = migrate.up_migrations().into_iter().range(latest, self.to);

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
            .report(&oks)?;
        }

        Ok(oks.into_iter().collect())
    }
}

/// Revert migrations in the given range if valid.
#[derive(Clone, DebugAsJson, DisplayAsJsonPretty, Serialize)]
pub struct Revert {
    to: i64,
    dryrun: bool,
}

impl Revert {
    /// Returns the `Revert` operation with target version `to`.
    pub fn new(to: i64) -> Self {
        Self { to, dryrun: false }
    }

    /// Return the report of the migrations that would be reverted.
    pub fn dryrun(self) -> Self {
        Self { dryrun: true, ..self }
    }
}

impl<T: Invertible> TernMigrateOp<T> for Revert {
    type Output = Completed;

    async fn exec(&self, migrate: &mut T) -> Report<Self::Output> {
        migrate.check_history_exists().await?;
        let latest = migrate.last_applied_id().await?.map(|id| id.version());
        let t = self.to;

        if latest.is_some_and(|l| l > t) {
            return Err(TernError::Invalid(format!(
                "Target version {t} does not exist!"
            )))?;
        }
        let set = migrate.down_migrations().into_iter().revert(t);

        let mut oks = Vec::new();

        for m in set {
            async {
                let ok = if self.dryrun {
                    let id = m.migration_id();
                    let query = m.query(migrate).await?;
                    MigrateOk::dryrun(id, Some(query))
                } else {
                    let applied = m.apply(migrate).await?;
                    m.post_apply(migrate, &applied).await?;
                    MigrateOk::reverted(applied)
                };
                oks.push(ok);
                Ok(())
            }
            .await
            .report(&oks)?;
        }

        Ok(oks.into_iter().collect())
    }
}
