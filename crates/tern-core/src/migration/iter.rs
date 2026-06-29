//! Iterable collections of migrations.
use std::collections::VecDeque;

use crate::context::MigrationContext;
use crate::migration::{DownMigration, Migration, UpMigration};

/// `UpMigrationSet` is a set of migrations that represent creating new versions
/// of the database.
#[derive(Clone)]
pub struct UpMigrationSet<Ctx> {
    inner: Vec<UpMigration<Ctx>>,
}

impl<Ctx: MigrationContext> UpMigrationSet<Ctx> {
    /// Create a new `MigrationSet`.
    pub fn new<T>(vs: T) -> Self
    where
        T: Into<Vec<UpMigration<Ctx>>>,
    {
        let mut inner = vs.into();
        inner.sort_by_key(|m| m.migration_id().version());
        Self { inner }
    }

    /// Return a slice of the migrations.
    ///
    /// The slice is sorted ascending by version.
    pub fn as_slice(&self) -> &[UpMigration<Ctx>] {
        self.inner.as_slice()
    }
}

impl<Ctx: MigrationContext> IntoIterator for UpMigrationSet<Ctx> {
    type IntoIter = UpIter<Ctx>;
    type Item = UpMigration<Ctx>;

    fn into_iter(self) -> Self::IntoIter {
        UpIter::new(self.inner)
    }
}

impl<'a, Ctx: MigrationContext> IntoIterator for &'a UpMigrationSet<Ctx> {
    type IntoIter = UpIterRef<'a, Ctx>;
    type Item = &'a UpMigration<Ctx>;

    fn into_iter(self) -> Self::IntoIter {
        UpIterRef::new(self.inner.as_slice())
    }
}

/// `DownMigrationSet` is a set of migrations that represent reverting the state
/// of the database to an earlier version.
#[derive(Clone)]
pub struct DownMigrationSet<Ctx> {
    inner: Vec<DownMigration<Ctx>>,
}

impl<Ctx: MigrationContext> DownMigrationSet<Ctx> {
    /// Create a new `DownMigrationSet`.
    pub fn new<T>(vs: T) -> Self
    where
        T: Into<Vec<DownMigration<Ctx>>>,
        Ctx: MigrationContext,
    {
        let mut inner = vs.into();
        // Ensure they are sorted in descending order.
        inner.sort_by_key(|m| -m.migration_id().version());
        Self { inner }
    }

    /// Return a slice of the down migrations.
    ///
    /// The slice is sorted descending by version.
    pub fn as_slice(&self) -> &[DownMigration<Ctx>] {
        self.inner.as_slice()
    }
}

impl<Ctx: MigrationContext> IntoIterator for DownMigrationSet<Ctx> {
    type IntoIter = DownIter<Ctx>;
    type Item = DownMigration<Ctx>;

    fn into_iter(self) -> Self::IntoIter {
        DownIter::new(self.inner)
    }
}

impl<'a, Ctx: MigrationContext + 'a> IntoIterator
    for &'a DownMigrationSet<Ctx>
{
    type IntoIter = DownIterRef<'a, Ctx>;
    type Item = &'a DownMigration<Ctx>;

    fn into_iter(self) -> Self::IntoIter {
        DownIterRef::new(self.inner.as_slice())
    }
}

/// Owned iterator for an [`UpMigrationSet`].
pub struct UpIter<Ctx> {
    inner: VecDeque<UpMigration<Ctx>>,
}

impl<Ctx: MigrationContext> UpIter<Ctx> {
    fn new<T>(inner: T) -> Self
    where
        T: Into<VecDeque<UpMigration<Ctx>>>,
    {
        Self { inner: inner.into() }
    }
}

impl<Ctx: MigrationContext> Iterator for UpIter<Ctx> {
    type Item = UpMigration<Ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop_front()
    }
}

/// Borrowed iterator for an [`UpMigrationSet`].
pub struct UpIterRef<'a, Ctx> {
    inner: &'a [UpMigration<Ctx>],
    idx: usize,
}

impl<'a, Ctx: MigrationContext + 'a> UpIterRef<'a, Ctx> {
    fn new(inner: &'a [UpMigration<Ctx>]) -> Self {
        Self { inner, idx: 0 }
    }
}

impl<'a, Ctx: MigrationContext + 'a> Iterator for UpIterRef<'a, Ctx> {
    type Item = &'a UpMigration<Ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        let it = self.inner.get(self.idx)?;
        self.idx += 1;
        Some(it)
    }
}

/// Owned iterator for a [`DownMigrationSet`].
pub struct DownIter<Ctx> {
    inner: VecDeque<DownMigration<Ctx>>,
}

impl<Ctx: MigrationContext> DownIter<Ctx> {
    fn new<T>(inner: T) -> Self
    where
        T: Into<VecDeque<DownMigration<Ctx>>>,
    {
        Self { inner: inner.into() }
    }
}

impl<Ctx: MigrationContext> Iterator for DownIter<Ctx> {
    type Item = DownMigration<Ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop_front()
    }
}

/// Borrowed iterator for a [`DownMigrationSet`].
pub struct DownIterRef<'a, Ctx> {
    inner: &'a [DownMigration<Ctx>],
    idx: usize,
}

impl<'a, Ctx: MigrationContext> DownIterRef<'a, Ctx> {
    fn new(inner: &'a [DownMigration<Ctx>]) -> Self {
        Self { inner, idx: inner.len() - 1 }
    }
}

impl<'a, Ctx: MigrationContext> Iterator for DownIterRef<'a, Ctx> {
    type Item = &'a DownMigration<Ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        let it = self.inner.get(self.idx)?;
        // The inner slice will be in descending order, so we want to traverse
        // _up_ through the indices.
        self.idx += 1;
        Some(it)
    }
}
