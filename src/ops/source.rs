use display_json::{DebugAsJson, DisplayAsJsonPretty};
use serde::Serialize;
use tern_core::migrate::ops::{FetchAll, Resolve};
use tern_core::migrate::{TernMigrate, TernMigrateOp};
use tern_core::migration::Migration;

use crate::report::{
    Completed, Incomplete, MigrateOk, OpResult, TryOpResult as _,
};

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
    type Success = Completed;
    type Error = Incomplete;

    async fn exec(&self, migrate: &mut T) -> OpResult<Self::Success> {
        migrate.check_history_exists().await?;

        Ok(FetchAll::<T>::new()
            .exec(migrate)
            .await?
            .into_iter()
            .filter(|m| {
                self.from.is_none_or(|v| v <= m.version())
                    && self.to.is_none_or(|v| v >= m.version())
            })
            .map(MigrateOk::applied)
            .collect::<Completed>())
    }
}

/// List the unapplied migrations.
#[derive(Clone, DebugAsJson, Default, DisplayAsJsonPretty, Serialize)]
pub struct Diff {
    render_queries: bool,
}

impl Diff {
    /// Returns default options for the `Diff` operation.
    pub fn new() -> Self {
        Diff::default()
    }

    /// Return the full query contents in the result
    pub fn render_queries(self) -> Self {
        Self { render_queries: true }
    }
}

impl<T: TernMigrate> TernMigrateOp<T> for Diff {
    type Success = Completed;
    type Error = Incomplete;

    async fn exec(&self, migrate: &mut T) -> OpResult<Self::Success> {
        migrate.check_history_exists().await?;
        let latest = migrate.last_applied_id().await?.map(|id| id.version());
        let set = migrate.up_migrations().iter_between(latest, None);

        if !self.render_queries {
            let output =
                set.map(MigrateOk::from_migration).collect::<Completed>();
            return Ok(output);
        }

        let mut oks = Vec::new();

        for m in set {
            let id = m.migration_id();
            let q = Resolve::new(&m).exec(migrate).await.incomplete(&oks)?;
            let ok = MigrateOk::unapplied(id, Some(q));
            oks.push(ok);
        }

        Ok(oks.into_iter().collect())
    }
}
