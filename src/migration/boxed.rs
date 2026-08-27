use futures_core::future::BoxFuture;
use std::collections::BTreeMap;
use tern_core::context::MigrationContext;
use tern_core::error::{TernError, TernResult};
use tern_core::migration::{Migration, MigrationId};
use tern_core::query::Query;

/// A dynamically-typed `Migration`.
pub struct MigrationBox<Ctx>(Box<dyn Migration<Ctx = Ctx>>);

impl<Ctx> MigrationBox<Ctx> {
    /// Create a new `MigrationBox`.
    pub fn new<M: Migration<Ctx = Ctx> + 'static>(
        inner: M,
    ) -> MigrationBox<Ctx> {
        Self(Box::new(inner))
    }
}

impl<Ctx: MigrationContext> Migration for MigrationBox<Ctx> {
    type Ctx = Ctx;

    fn migration_id(&self) -> &MigrationId {
        self.0.migration_id()
    }

    fn query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Query>> {
        self.0.query(ctx)
    }

    fn revert_query<'a>(
        &'a self,
        ctx: &'a mut Self::Ctx,
    ) -> BoxFuture<'a, TernResult<Option<Query>>> {
        self.0.revert_query(ctx)
    }
}

/// A `MigrationBoxSet` is an iterator over dynamically-typed `Migration`s.
pub struct MigrationBoxSet<Ctx> {
    inner: BTreeMap<i64, MigrationBox<Ctx>>,
}

impl<Ctx> Default for MigrationBoxSet<Ctx> {
    fn default() -> Self {
        Self { inner: BTreeMap::new() }
    }
}

impl<Ctx: MigrationContext> MigrationBoxSet<Ctx> {
    /// New migration set from an iterator of [`MigrationBox`] values.
    ///
    /// Use [`MigrationBoxSet::try_insert`] for perhaps a more ergonomic way to
    /// build the value.
    pub fn try_new<I>(iter: I) -> TernResult<Self>
    where
        I: IntoIterator<Item = MigrationBox<Ctx>>,
    {
        iter.into_iter().try_fold(Self::default(), |mut acc, m| {
            let v = m.version();
            if let Some(dup) = acc.inner.insert(v, m) {
                return Err(TernError::Duplicate(dup.version()));
            }
            Ok(acc)
        })
    }

    /// New migration set from an iterator of generic [`Migration`]s.
    pub fn try_from_iter<I>(iter: I) -> TernResult<Self>
    where
        I: IntoIterator<Item: Migration<Ctx = Ctx> + 'static>,
    {
        iter.into_iter().try_fold(Self::default(), |acc, m| acc.try_insert(m))
    }

    /// Insert the migration into this migration set.
    ///
    /// # Errors
    ///
    /// An error is returned if the set contains a migration with the same
    /// version.
    pub fn try_insert<M: Migration<Ctx = Ctx> + 'static>(
        mut self,
        migration: M,
    ) -> TernResult<Self> {
        let key = migration.version();
        let value = MigrationBox::new(migration);
        if let Some(v) = self.inner.insert(key, value) {
            return Err(TernError::Duplicate(v.version()));
        }
        Ok(self)
    }

    /// Return the number of migrations in this migration set.
    pub fn size(&self) -> usize {
        self.inner.len()
    }

    /// Return the latest version added.
    ///
    /// `None` if this migration set has no migrations.
    pub fn version(&self) -> Option<i64> {
        self.inner.last_key_value().and_then(|(k, _)| Some(*k))
    }
}

/// `MigrationBoxSet` iterator.
pub mod migration_box_set {
    use std::collections::btree_map::IntoIter;

    use super::*;

    /// An iterator of `MigrationBox`.
    pub struct IterMigrate<Ctx>(IntoIter<i64, MigrationBox<Ctx>>);

    impl<Ctx: MigrationContext> IntoIterator for MigrationBoxSet<Ctx> {
        type IntoIter = IterMigrate<Ctx>;
        type Item = MigrationBox<Ctx>;

        fn into_iter(self) -> Self::IntoIter {
            IterMigrate(self.inner.into_iter())
        }
    }

    impl<Ctx: MigrationContext> Iterator for IterMigrate<Ctx> {
        type Item = MigrationBox<Ctx>;

        fn next(&mut self) -> Option<Self::Item> {
            let (_, v) = self.0.next()?;
            Some(v)
        }
    }

    impl<Ctx: MigrationContext> DoubleEndedIterator for IterMigrate<Ctx> {
        fn next_back(&mut self) -> Option<Self::Item> {
            let (_, v) = self.0.next_back()?;
            Some(v)
        }
    }
}
