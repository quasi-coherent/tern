use chrono::Utc;
use std::marker::PhantomData;

use crate::error::{TernError, TernResult};
use crate::executor::Executor;
use crate::migrate::{Invertible, TernMigrate, TernMigrateOp};
use crate::migration::{
    Applied, DownMigration, Migration, PostApply as _, Query, UpMigration,
};

/// An operation for fetching all applied migrations.
pub struct FetchAll<T>(PhantomData<T>);

impl<T: TernMigrate> FetchAll<T> {
    /// New operation.
    pub fn new() -> FetchAll<T> {
        Self::default()
    }
}

impl<T: TernMigrate> TernMigrateOp<T> for FetchAll<T> {
    type Success = Vec<Applied>;
    type Error = TernError;

    async fn exec(&self, migrate: &mut T) -> TernResult<Self::Success> {
        migrate.all_applied().await
    }
}

impl<T: TernMigrate> Default for FetchAll<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// An operation for updating an applied migration.
#[derive(Debug)]
pub struct UpdateApplied<'a>(&'a Applied);

impl<'a> UpdateApplied<'a> {
    /// `UpdateApplied` operation with new values `applied`.
    pub fn new(applied: &'a Applied) -> Self {
        Self(applied)
    }
}

impl<'a, T: TernMigrate> TernMigrateOp<T> for UpdateApplied<'a> {
    type Success = ();
    type Error = TernError;

    async fn exec(&self, migrate: &mut T) -> TernResult<Self::Success> {
        migrate.update_applied(self.0).await
    }
}

/// An operation for resolving a migration's query.
#[derive(Debug)]
pub struct Resolve<'a, M>(&'a M);

impl<'a, M> Resolve<'a, M> {
    /// New `Resolve` operation for this migration `M`.
    pub fn new(migration: &'a M) -> Self {
        Self(migration)
    }
}

impl<'a, M, T> TernMigrateOp<T> for Resolve<'a, M>
where
    T: TernMigrate,
    M: Migration<Ctx = T>,
{
    type Success = Query;
    type Error = TernError;

    async fn exec(&self, migrate: &mut T) -> TernResult<Self::Success> {
        self.0.query(migrate).await
    }
}

/// An operation for applying one up migration.
#[derive(Debug)]
pub struct Up<'a, T>(&'a UpMigration<T>);

impl<'a, T> Up<'a, T> {
    /// New `Up` operation for this migration `M`.
    pub fn new(migration: &'a UpMigration<T>) -> Self {
        Self(migration)
    }
}

impl<'a, T> TernMigrateOp<T> for Up<'a, T>
where
    T: TernMigrate,
{
    type Success = Applied;
    type Error = TernError;

    async fn exec(
        &self,
        migrate: &mut T,
    ) -> Result<Self::Success, Self::Error> {
        let start = Utc::now();
        let id = self.0.migration_id();

        log::debug!(id:%; "resolving migration query");
        let q = Resolve::new(self.0).exec(migrate).await?;

        log::debug!(id:%, query:% = q; "applying query");
        migrate.executor_mut().send(&q).await?;

        log::debug!(id:%; "applied migration");
        let content = q.to_string();
        let applied = Applied::new(&id, content, start);

        self.0.post_apply(migrate, &applied).await?;
        Ok(applied)
    }
}

/// An operation for applying one down migration.
#[derive(Debug)]
pub struct Down<'a, T>(&'a DownMigration<T>);

impl<'a, T> Down<'a, T> {
    /// New `Down` operation for this migration `M`.
    pub fn new(migration: &'a DownMigration<T>) -> Self {
        Self(migration)
    }
}

impl<'a, T> TernMigrateOp<T> for Down<'a, T>
where
    T: Invertible,
{
    type Success = Applied;
    type Error = TernError;

    async fn exec(
        &self,
        migrate: &mut T,
    ) -> Result<Self::Success, Self::Error> {
        let start = Utc::now();
        let id = self.0.migration_id();

        log::debug!(id:%; "resolving down migration query");
        let q = Resolve::new(self.0).exec(migrate).await?;

        log::debug!(id:%, query:% = q; "reverting migration");
        migrate.executor_mut().send(&q).await?;

        log::debug!(id:%; "reverted migration");
        let content = q.to_string();
        let applied = Applied::new(&id, content, start);

        self.0.post_apply(migrate, &applied).await?;
        Ok(applied)
    }
}

/// `TernMigrateOp` for a type that can be converted into one.
pub struct TryMigrateOp<'a, Op, V>(&'a V, PhantomData<Op>);

impl<'a, Op, V> TryMigrateOp<'a, Op, V> {
    /// New from a borrow of `V`.
    pub fn new(v: &'a V) -> TryMigrateOp<'a, Op, V> {
        TryMigrateOp(v, PhantomData)
    }
}

impl<'a, T, Op, V> TernMigrateOp<T> for TryMigrateOp<'a, Op, V>
where
    T: TernMigrate,
    V: Send + Sync,
    Op: TernMigrateOp<T, Error = TernError> + TryFrom<&'a V, Error = TernError>,
{
    type Success = <Op as TernMigrateOp<T>>::Success;
    type Error = TernError;

    async fn exec(&self, migrate: &mut T) -> TernResult<Self::Success> {
        let m: Op = Op::try_from(self.0)?;
        m.exec(migrate).await
    }
}
