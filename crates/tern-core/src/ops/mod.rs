//! Operations with a migration context.
//!
//! This module extends a `MigrationContext` by defining [`MigrationOp`], which
//! is an operation to carry out in the course of a migration app, and then
//! implements it for several canonical forms over an arbitrary context.
//!
//! A migration app would typically collect some combination of these into some
//! top-level API.
use futures_core::future::Future;

pub mod crud;
pub mod ddl;
pub mod migration;

/// An operation related to migrations.
pub trait MigrationOp<T> {
    /// The type of value returned by the operation.
    type Output;

    /// Execute the operation.
    fn exec(self, input: T) -> impl Future<Output = Self::Output> + Send;
}
