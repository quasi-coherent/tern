//! # partition
//!
//! This example shows a real use case where a migration is vastly simplified
//! using `tern`.
//!
//! # Introduction
//!
//! In postgres, it is possible to create a [partitioned] table.  A partitioned
//! table is not a normal table.  It is made up from a parent table that has no
//! data and many child tables that partition the records according to some
//! definition.
//!
//! A partitioned table may have indices built on it.  But like a normal table,
//! a `CREATE INDEX blah ON...` acquires an exclusive lock and relinquishes it
//! only when the index is built.  This is very frequently unacceptable, so a
//! strategy to create the index with little or no downtime is usually to create
//! the index with the `CONCURRENTLY` keyword.  This does something like a
//! "soft" index build, but importantly it permits concurrent writes.
//!
//! # Strategy
//!
//! Important too is that is cannot happen in a database transaction. Also
//! importantly... It is not allowed on partitioned tables.
//!
//! So the strategy is much more elaborate:
//!
//! 1. `CREATE INDEX whatever ON ONLY parent_table` creates the index on the
//!    parent table and not on the child tables.  This is a metadata-only
//!    operation and it applies instantly.
//! 2. Iterate over all the child tables.  For each, a) `CREATE INDEX
//!    CONCURRENTLY child_k_of_n_whatever ON child_k_of_n` creates the index
//!    named for the child table having the same definition as the one we
//!    created on the parent. b) `ALTER INDEX whatever ATTACH PARTITION
//!    child_table_k_of_n_whatever` attaches the child index to the parent one.
//!    This establishes the inheritance relationship that is needed for the
//!    index to exist on the partitioned table as a whole.
//!
//! But a problem with this is that it is not known _a priori_ what the names of
//! the child tables are.  Partitions are attached and detached all the time,
//! and while names do follow a pattern, it's error-prone to try to interpolate
//! the dynamic values.
//!
//! The alternative is to write SQL to run SQL that has had the records from the
//! former transformed into index build queries in the latter.  This amounts to
//! asking PostgreSQL to `exec` a query from a string, which is an uncommon
//! thing to do.  It's possible, but clearly unappealing.
//!
//! Instead we're going to define a `TernApp` that has a context capable of
//! fetching the complete list of currently attached partitions into a Rust
//! value `Vec<Partition>`, which is easy to iterate over and format the query.
//!
//! [partitioned]: https://www.postgresql.org/docs/current/ddl-partitioning.html
mod migrations;
pub use migrations::PartitionExample;
