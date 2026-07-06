use futures_core::future::BoxFuture;
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
    inner: Vec<MigrationBox<Ctx>>,
    version: i64,
}

impl<Ctx> Default for MigrationBoxSet<Ctx> {
    fn default() -> Self {
        Self { inner: Vec::new(), version: 0 }
    }
}

impl<Ctx: MigrationContext> MigrationBoxSet<Ctx> {
    /// New empty migration set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add the next migration to the set.
    ///
    /// # Errors
    ///
    /// This returns an error if the version is not the next version.
    pub fn add<M: Migration<Ctx = Ctx> + 'static>(
        mut self,
        migration: M,
    ) -> TernResult<Self> {
        if let v = migration.version()
            && let e = self.version + 1
            && v != e
        {
            return Err(TernError::Invalid(format!("got {v} expected {e}")));
        }
        self.inner.push(MigrationBox::new(migration));
        self.version += 1;
        Ok(self)
    }

    /// Add the next migration without checking validity of the version.
    ///
    /// # Panics
    ///
    /// Panics if the version is not the next version.
    pub fn add_unchecked<M: Migration<Ctx = Ctx> + 'static>(
        self,
        migration: M,
    ) -> Self {
        self.add(migration).expect("unchecked version mismatch")
    }

    /// Return the number of migrations in this migration set.
    pub fn size(&self) -> usize {
        self.inner.len()
    }

    /// Return the latest version added.
    pub fn version(&self) -> i64 {
        self.version
    }
}

impl<Ctx: MigrationContext, M: Migration<Ctx = Ctx> + 'static> FromIterator<M>
    for MigrationBoxSet<Ctx>
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = M>,
    {
        let (version, inner) =
            iter.into_iter().fold((0, Vec::new()), |(v, mut acc), m| {
                let newv = if v < m.version() { m.version() } else { v };
                let boxm = MigrationBox::new(m);
                acc.push(boxm);
                (newv, acc)
            });
        Self { inner, version }
    }
}

impl<Ctx: MigrationContext> IntoIterator for MigrationBoxSet<Ctx> {
    type IntoIter = IterMigrate<Ctx>;
    type Item = MigrationBox<Ctx>;

    fn into_iter(self) -> Self::IntoIter {
        IterMigrate(self.inner.into_iter())
    }
}

/// A set of up migrations.
pub struct IterMigrate<Ctx>(std::vec::IntoIter<MigrationBox<Ctx>>);

impl<Ctx: MigrationContext> Iterator for IterMigrate<Ctx> {
    type Item = MigrationBox<Ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<Ctx: MigrationContext> DoubleEndedIterator for IterMigrate<Ctx> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}
