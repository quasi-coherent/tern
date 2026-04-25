//! Collections over a set of migration.
//!
//! This module contains the [`UpMigrationSet`] type for constructing new
//! versions of the database, and the [`DownMigrationSet`] for reverting to an
//! earlier version.  It also contains iterators for these types to simplify
//! operations over a range of versions.
use std::collections::VecDeque;
use tern_core::context::MigrationContext;
use tern_core::migration::Migration;

use crate::migration::{DownMigration, UpMigration};

/// `UpMigrationSet` is a set of migrations that represent creating new versions
/// of the database.
#[derive(Clone)]
pub struct UpMigrationSet<Ctx> {
    inner: Vec<UpMigration<Ctx>>,
}

impl<Ctx> UpMigrationSet<Ctx> {
    /// Create a new `MigrationSet`.
    pub fn new<T>(vs: T) -> Self
    where
        T: Into<Vec<UpMigration<Ctx>>>,
        Ctx: MigrationContext,
    {
        let mut inner = vs.into();
        inner.sort_by_key(|m| m.migration_id().version());
        Self { inner }
    }
}

impl<Ctx> IntoIterator for UpMigrationSet<Ctx> {
    type Item = UpMigration<Ctx>;
    type IntoIter = UpIter<Ctx>;

    fn into_iter(self) -> Self::IntoIter {
        UpIter::new(self.inner)
    }
}

impl<'a, Ctx> IntoIterator for &'a UpMigrationSet<Ctx> {
    type Item = &'a UpMigration<Ctx>;
    type IntoIter = UpIterRef<'a, Ctx>;

    fn into_iter(self) -> Self::IntoIter {
        UpIterRef::new(self.inner.as_slice())
    }
}

/// Owned iterator for an [`UpMigrationSet`].
pub struct UpIter<Ctx> {
    inner: VecDeque<UpMigration<Ctx>>,
}

impl<Ctx> UpIter<Ctx> {
    fn new<T>(inner: T) -> Self
    where
        T: Into<VecDeque<UpMigration<Ctx>>>,
    {
        Self { inner: inner.into() }
    }
}

impl<Ctx> Iterator for UpIter<Ctx> {
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

impl<'a, Ctx> UpIterRef<'a, Ctx> {
    fn new(inner: &'a [UpMigration<Ctx>]) -> Self {
        Self { inner, idx: 0 }
    }
}

impl<'a, Ctx> Iterator for UpIterRef<'a, Ctx> {
    type Item = &'a UpMigration<Ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        let it = self.inner.get(self.idx)?;
        self.idx += 1;
        Some(it)
    }
}

/// `DownMigrationSet` is a set of migrations that represent reverting the state
/// of the database to an earlier version.
#[derive(Clone)]
pub struct DownMigrationSet<Ctx> {
    inner: Vec<DownMigration<Ctx>>,
}

impl<Ctx> DownMigrationSet<Ctx> {
    /// Create a new `DownMigrationSet`.
    pub fn new<T>(vs: T) -> Self
    where
        T: Into<Vec<DownMigration<Ctx>>>,
        Ctx: MigrationContext,
    {
        let mut inner = vs.into();
        inner.sort_by_key(|m| m.migration_id().version());
        Self { inner }
    }
}

impl<Ctx> IntoIterator for DownMigrationSet<Ctx> {
    type Item = DownMigration<Ctx>;
    type IntoIter = DownIter<Ctx>;

    fn into_iter(self) -> Self::IntoIter {
        DownIter::new(self.inner)
    }
}

impl<'a, Ctx> IntoIterator for &'a DownMigrationSet<Ctx> {
    type Item = &'a DownMigration<Ctx>;
    type IntoIter = DownIterRef<'a, Ctx>;

    fn into_iter(self) -> Self::IntoIter {
        DownIterRef::new(self.inner.as_slice())
    }
}

/// Owned iterator for an [`DownMigrationSet`].
pub struct DownIter<Ctx> {
    inner: Vec<DownMigration<Ctx>>,
}

impl<Ctx> DownIter<Ctx> {
    fn new<T>(inner: T) -> Self
    where
        T: Into<Vec<DownMigration<Ctx>>>,
    {
        Self { inner: inner.into() }
    }
}

impl<Ctx> Iterator for DownIter<Ctx> {
    type Item = DownMigration<Ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop()
    }
}

/// Borrowed iterator for a [`DownMigrationSet`].
pub struct DownIterRef<'a, Ctx> {
    inner: &'a [DownMigration<Ctx>],
    idx: usize,
}

impl<'a, Ctx> DownIterRef<'a, Ctx> {
    fn new(inner: &'a [DownMigration<Ctx>]) -> Self {
        Self { inner, idx: inner.len() - 1 }
    }
}

impl<'a, Ctx> Iterator for DownIterRef<'a, Ctx> {
    type Item = &'a DownMigration<Ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        let it = self.inner.get(self.idx)?;
        self.idx -= 1;
        Some(it)
    }
}
