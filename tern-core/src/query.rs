//! Module containing the [`Query`] for a migration.
//!
//! # Description
//!
//! `Query` is the value that holds the SQL to be applied during a migration
//! run.  It is an enumeration of two variants of a migration query: a single
//! [`Statement`], or the collection [`Statements`].
//!
//! A `Statement` is a query that may have more than one constituent SQL query,
//! but it is prepared as one query and executed in a database transaction
//! together with the query to save the applied migration in the history table.
//!
//! A query with `Statements`, in contrast, is ran by executing each statement
//! in order, exiting on the first error encountered.  So in particular, the
//! full `Statements` value is not ran in a transaction and may result in
//! partial success.  The history table is updated if there were no errors.
//!
//! This can lead to an inconsistent state of the history table if statements
//! succeed but the history update statement fails.  In this case, it may be
//! necessary to do a "soft" update of the history table.
//!
//! A `Query` with `Statements` fits the use case of a migration with SQL that
//! cannot, or otherwise should not, be ran in a database transaction.  For
//! instance, it is an error to run certain index builds in a transaction in
//! PostgreSQL.
//!
//! ## `tern` Annotations
//!
//! To facilitate a SQL source file containing `Statements` for a migration,
//! `tern` understands certain "magic" annotations, which are comments with
//! keywords found in the SQL file.
//!
//! The main annotation is `tern:noTransaction`, which must appear on the first
//! line in a comment.  If this is found, the rest of the contents are
//! interpreted as groups in a collection of `Statements`.  Create a statement
//! group by enclosing statements between the tags `tern:begin` and `tern:end`.
//!
//! **Note**: A statement group consisting of a single query may elect to omit
//! these tags, but be cautious with this--a "statement" in this context is
//! just any text that ends in a semicolon.  There is no attempt made to parse
//! an AST from SQL because of the enormous complication that introduces,
//! multiplied by the number of SQL variants this library should be able to
//! support.
//!
//! For instance, a `CREATE FUNCTION` statement may not work as-is and may
//! require the statement group tags to surround it.
//!
//! # Example
//!
//! This is an example of a SQL source file that we want to run as a collection
//! of `Statements`.
//!
//! ```sql
//! -- tern:noTransaction
//! -- The previous line means we will create groups of SQL statements.
//!
//! -- The following line opens a statement group:
//! -- tern:begin
//! CREATE TABLE blah.whatev_fast (LIKE blah.whatev INCLUDING CONSTRAINTS);
//!
//! -- This index build will happen in a transaction with the `CREATE TABLE...`
//! -- statement.
//! CREATE INDEX blah_whatev_fast_ca_brin_idx
//!   ON blah.whatev_fast USING brin (created_at)
//!   WITH (pages_per_range=64);
//!
//! -- Make it efficient:
//! SELECT brin_summarize_new_values('blah_whatev_fast_ca_brin_idx');
//!
//! -- Now we're going to close the group with:
//! -- tern:end
//!
//! A group of one:
//! -- tern:begin
//! SELECT 'Everybody stop!';
//! -- tern:end
//!
//! -- Or omit the tags:
//! VACUUM ANALYZE blah.whatev_fast;
//! ```
use futures_core::Future;
use std::fmt::{self, Display, Formatter, Write as _};
use std::ops::{Deref, DerefMut};
use std::slice::Iter;

use crate::error::{TernError, TernResult};
use crate::context::MigrationContext;

/// A `Query` builder method for a `Migration`.
///
/// The trait `QueryBuilder` can be used as a helper for implementing the
/// [`Migration`] trait by supplying its [`resolve_query`] method.
///
/// [`resolve_query`]: crate::migration::Migration::resolve_query
pub trait QueryBuilder<Ctx: MigrationContext> {
    /// Use the specified context to build the `Query`.
    fn build(&self, ctx: &mut Ctx) -> impl Future<Output = TernResult<Query>> + Send;
}

/// `Query` holds the SQL statements to run when applying a migration.
#[derive(Clone, Debug)]
pub enum Query {
    One(Statement),
    Many(Statements),
}

impl From<Statement> for Query {
    fn from(value: Statement) -> Self {
        Self::One(value)
    }
}

impl From<Statements> for Query {
    fn from(value: Statements) -> Self {
        Self::Many(value)
    }
}

impl Display for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::One(s) => s.fmt(f),
            Self::Many(ss) => ss.fmt(f),
        }
    }
}

/// `Statement` holds one or more SQL queries to be sent and executed by the
/// database as one query.
#[derive(Debug, Clone, Default)]
pub struct Statement(String);

impl Statement {
    /// Create an empty statement.
    pub fn new_empty() -> Self {
        Self::default()
    }

    /// New `Statement` from a SQL string.
    pub fn from_sql<T: Into<String>>(sql: T) -> Self {
        Self(Statement::ensure_term(sql))
    }

    /// Push a SQL string to the statement.
    ///
    /// No validation is done other than ensuring it ends with a semicolon.
    ///
    /// # Errors
    ///
    /// This returns an error if writing to the internal buffer fails.
    pub fn push_sql<T: Into<String>>(&mut self, sql: T) -> TernResult<()> {
        let statement = Statement::ensure_term(sql);
        writeln!(&mut self.0, "{statement}")?;
        Ok(())
    }

    /// Ensures the raw SQL ends in a semicolon.
    pub(crate) fn ensure_term<T: Into<String>>(sql: T) -> String {
        let sql = sql.into();
        let sql_str = sql.trim();
        if sql_str.ends_with(";") {
            sql_str.to_string()
        } else {
            format!("{sql_str};")
        }
    }
}

impl Deref for Statement {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Statement {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Statements> for Statement {
    fn from(value: Statements) -> Self {
        let sql = value.to_string();
        Self::from_sql(sql)
    }
}

impl Display for Statement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// `Statements` holds an ordered collection of [`Statement`]s to be sent and
/// executed sequentially by the database.
#[derive(Debug, Clone, Default)]
pub struct Statements {
    buf: Option<Statement>,
    inner: Vec<Statement>,
}

impl Statements {
    /// Create a new empty collection of statements.
    pub fn new_empty() -> Self {
        Self::default()
    }

    /// Begin a new statement.
    ///
    /// # Errors
    ///
    /// This returns an error if there is already an open statement.
    pub fn begin(&mut self) -> TernResult<()> {
        if let existing = self.buf.replace(Statement::new_empty())
            && existing.is_some()
        {
            return Err(TernError::QueryBuilder(
                "called `begin` with existing open statement",
            ))?;
        }
        Ok(())
    }

    /// Push a SQL string to the `Statements`, either by adding it to an open
    /// `Statement` if it exists, or by creating a `Statement` from it alone.
    ///
    /// # Errors
    ///
    /// This returns an error if writing to the internal buffer fails.
    pub fn push_sql<T: Into<String>>(&mut self, sql: T) -> TernResult<()> {
        if let Some(statement) = self.buf.as_mut() {
            statement.push_sql(sql)?;
        } else {
            let statement = Statement::from_sql(sql);
            self.inner.push(statement);
        }
        Ok(())
    }

    /// Close a statement and add it to the collection.
    ///
    /// # Errors
    ///
    /// This returns an error if there is no statement open.
    pub fn close(&mut self) -> TernResult<()> {
        let Some(statement) = self.buf.take() else {
            return Err(TernError::QueryBuilder(
                "called `close` with no open statement",
            ))?;
        };
        self.inner.push(statement);
        Ok(())
    }

    /// Returns whether there is a statement that has not been closed yet.
    pub fn has_open_statement(&self) -> bool {
        self.buf.is_some()
    }

    /// Returns an iterator over the collection of statements.
    pub fn iter(&self) -> Iter<'_, Statement> {
        self.inner.iter()
    }
}

impl Display for Statements {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner
            .iter()
            .map(Deref::deref)
            .collect::<Vec<_>>()
            .join("\n\n")
            .fmt(f)
    }
}
