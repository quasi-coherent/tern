use display_json::{DebugAsJson, DisplayAsJsonPretty};
use serde::Serialize;
use tern_core::migration::Migration;

use crate::migrate::TernMigrate;
use crate::migration::types::*;
use crate::operation::TernMigrateOp;
use crate::report::{Completed, Incomplete, MigrateOk, Report};

/// List the applied migrations.
#[derive(Clone, DebugAsJson, Default, DisplayAsJsonPretty, Serialize)]
pub struct List {
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<i64>,
}

impl List {
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
}

impl<T: TernMigrate> TernMigrateOp<T> for List {
    type Output = Completed;

    async fn exec(&self, migrate: &mut T) -> Report<Self::Output> {
        migrate.check_history_exists().await?;
        super::check_range(self.from, self.to)?;
        let output = migrate
            .all_applied()
            .await
            .map_err(Incomplete::from)?
            .into_iter()
            .map(MigrateOk::applied)
            .collect::<Completed>();
        Ok(output)
    }
}

/// List the unapplied migrations.
#[derive(Clone, DebugAsJson, Default, DisplayAsJsonPretty, Serialize)]
pub struct Diff {
    resolve_queries: bool,
}

impl Diff {
    /// Returns default options for the `Diff` operation.
    pub fn new() -> Self {
        Diff::default()
    }

    /// For unapplied migrations that are dynamically defined, resolve the query
    /// for the output.
    ///
    /// A placeholder will appear for the query content by default.
    pub fn resolve_queries(self) -> Self {
        Self { resolve_queries: true }
    }
}

impl<T: TernMigrate> TernMigrateOp<T> for Diff {
    type Output = Completed;

    async fn exec(&self, migrate: &mut T) -> Report<Self::Output> {
        migrate.check_history_exists().await?;
        let latest = migrate.last_applied_id().await?.map(|id| id.version());
        let set = migrate.up_migrations().into_iter().range(latest, None);

        if !self.resolve_queries {
            let output =
                set.map(MigrateOk::from_migration).collect::<Completed>();
            return Ok(output);
        }

        let mut oks = Vec::new();

        for m in set {
            async {
                let q = m.query(migrate).await?;
                let id = m.migration_id();
                let ok = MigrateOk::unapplied(id, Some(q));
                oks.push(ok);
                Ok(())
            }
            .await
            .report(&oks)?;
        }

        Ok(oks.into_iter().collect())
    }
}
