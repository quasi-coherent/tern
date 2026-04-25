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
//! A query with `Statements`, in contrast, is ran by executing each element of
//! the collection in order, exiting on the first error.
//!
//! A `Query` with `Statements` fits the use case of a migration with SQL that
//! cannot, or otherwise should not, be ran in a database transaction.  For
//! instance, it is an error to run certain index builds in a transaction in
//! PostgreSQL.
//!
//! ## Source annotations
//!
//! To facilitate a SQL source file containing `Statements` for a migration,
//! `tern` understands certain "magic" annotations, which are comments with
//! keywords found in the SQL file.
//!
//! The main annotation is `tern:noTransaction`, which must appear on the first
//! line in a comment.  This instructs `tern` to run the migration query outside
//! of a database transaction, which means that the query must be split into a
//! `Statements` collection.
//!
//! It is possible to indicate that some group of statements in the file should
//! be ran in a transaction even if the entire query should not.  This is
//! achieved with the annotations `tern:begin` and `tern:end`, which should
//! surround a section that needs to be ran in a transaction.
//!
//! _Note_: Parsing SQL is hard...  If issues arise where a query is not being
//! split into statements correctly, first try to see if the syntax can be
//! adjusted to be easier to parse without changing the meaning.  For instance,
//! complicated use of commenting can be the cause of such issues.  If that
//! doesn't work, you can provide a hint to the dialect of SQL being used,
//! which may resolve the problem.  This is done with the `noTransaction`
//! annotation:
//!
//! ```sql
//! -- tern:noTransaction,postgres declares the file to contain postgres syntax.
//! -- Other values that are accepted are `mysql` and `sqlite`.  The default is
//! -- "postgres".
//! ```
//!
//! And if that doesn't work, please open an issue.
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
//! -- Omitting the tags implies a group of one statement.
//! SELECT 'Done!';
//! ```
use futures_core::Future;
use regex::Regex;
use sql_splitter::parser::{Parser, SqlDialect};
use std::fmt::{self, Display, Formatter, Write as _};
use std::io::{BufRead as _, BufReader, Error as IoError, Read};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::slice::Iter;
use std::sync::OnceLock;

use crate::context::MigrationContext;
use crate::error::{TernError, TernResult};

// Default capacity of `std::io::BufReader`.
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

/// A `Query` builder method for a `Migration`.
///
/// The trait `QueryBuilder` can be used as a helper for implementing the
/// [`Migration`] trait by supplying its [`resolve_query`] method.
///
/// [`resolve_query`]: crate::migration::Migration::resolve_query
pub trait QueryBuilder<Ctx: MigrationContext> {
    /// Use the specified context to build the `Query`.
    fn build(
        &self,
        ctx: &mut Ctx,
    ) -> impl Future<Output = TernResult<Query>> + Send;
}

/// `Query` holds the SQL statements to run when applying a migration.
#[derive(Clone, Debug)]
pub enum Query {
    One(Statement),
    Many(Statements),
}

impl Query {
    /// Create a `Query` from a path to a SQL file.
    pub fn read_file<P: AsRef<Path>>(path: P) -> TernResult<Self> {
        let f = std::fs::File::open(path)?;

        let mut reader = BufReader::new(f);
        let mut sql = String::new();
        let _ = reader.read_line(&mut sql)?;
        let note = Annotation::new(&sql);

        if note.in_tx() {
            let mut f = reader.into_inner();
            let _ = f.read_to_string(&mut sql)?;
            let stat = Statement::new(sql.into());
            Ok(stat.into())
        } else {
            Self::as_many(reader, note.dialect)
        }
    }

    /// Create a `Query` directly from a SQL string.
    pub fn from_sql<T: AsRef<str>>(sql: T) -> TernResult<Self> {
        let sql = sql.as_ref();

        let mut lines = sql.trim().lines();
        let Some(header) = lines.next() else {
            return Err(TernError::QueryBuilder("found empty input"));
        };
        let note = Annotation::new(header);

        if note.in_tx() {
            let stat = Statement::new(sql.into());
            Ok(stat.into())
        } else {
            Self::as_many(sql.as_bytes(), note.dialect)
        }
    }

    /// Create an empty `Statement`.
    pub fn new_statement() -> Statement {
        Statement::new_empty()
    }

    /// Create an empty `Statements`.
    pub fn new_statements() -> Statements {
        Statements::new_empty()
    }

    /// Split the reader's source bytes into a `Statements` collection.
    fn as_many<R: Read>(
        reader: R,
        dialect: Option<SqlDialect>,
    ) -> TernResult<Self> {
        let d = dialect.unwrap_or(SqlDialect::Postgres);
        let mut parser = Parser::with_dialect(reader, DEFAULT_BUF_SIZE, d);
        let mut stats = Statements::new_empty();

        while let Some(stat_bytes) = parser.read_statement()? {
            let sql = String::from_utf8(stat_bytes).map_err(IoError::other)?;
            stats.push_sql(sql)?;
        }

        Ok(stats.into())
    }
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
    /// Create a new statement from a string verbatim.
    pub fn new(sql: String) -> Self {
        Self(sql)
    }

    /// Create an empty statement.
    pub fn new_empty() -> Self {
        Self::default()
    }

    /// Push a SQL string to the statement.
    ///
    /// No validation is done other than ensuring it is nonempty and ends with a
    /// semicolon.
    ///
    /// # Errors
    ///
    /// This returns an error if writing to the internal buffer fails.
    pub fn push_sql<T: AsRef<str>>(&mut self, sql: T) -> TernResult<()> {
        let sql_str = sql.as_ref().trim();
        if sql_str.is_empty() {
            return Ok(());
        }
        if sql_str.ends_with(";") {
            writeln!(&mut self.0, "{sql_str}")
        } else {
            writeln!(&mut self.0, "{sql_str};")
        }
        .map_err(IoError::other)?;

        Ok(())
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
        Self::new(sql)
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

    /// Returns an iterator over the collection of statements.
    pub fn iter(&self) -> Iter<'_, Statement> {
        self.inner.iter()
    }

    /// Returns the collection of statements in a slice.
    pub fn as_slice(&self) -> &[Statement] {
        self.inner.as_slice()
    }

    /// Begin a new statement.
    ///
    /// This should only be used as an alternative to the special annotations
    /// `tern:begin` and `tern:end`.
    ///
    /// # Errors
    ///
    /// This returns an error if there is already an open statement.
    pub fn begin(&mut self) -> TernResult<()> {
        let existing = self.buf.replace(Statement::new_empty());
        if existing.is_some() {
            Err(TernError::QueryBuilder(
                "called `begin` with existing open statement",
            ))
        } else {
            Ok(())
        }
    }

    /// Push a SQL string to the `Statements`, either by adding it to an open
    /// `Statement` if it exists, or by creating a `Statement` from it alone.
    ///
    /// If the input string opens with the annotation `tern:begin`, this will
    /// open a new statement and add subsequent SQL to it until is is ended.
    ///
    /// This should only be used as an alternative to calling the methods
    /// [`begin`](Self::begin) and [`end`](Self::end) manually.  So a statement
    /// can be closed only by passing a value that _begins_ with the annotation
    /// `tern:end`, ending the open statement before handling the present value.
    ///
    /// # Errors
    ///
    /// This returns an error if writing to the internal buffer fails, or if the
    /// statement is open (resp. closed) and the input starts with `tern:begin`
    /// (resp. `tern:end`).
    pub fn push_sql<T: AsRef<str>>(&mut self, sql: T) -> TernResult<()> {
        let mut lines = sql.as_ref().trim().lines();

        match lines.next() {
            Some(l) if l.contains("tern:begin") => {
                self.begin()?;
                // We literally just put something there.
                let stat = self.buf.as_mut().expect("inconceivable");
                let rest: &str = &lines.collect::<Vec<_>>().join("\n");
                stat.push_sql(rest)?;
            },
            Some(l) if l.contains("tern:end") => {
                self.end()?;
                let rest: &str = &lines.collect::<Vec<_>>().join("\n");
                return self.push_sql(rest);
            },
            Some(_) => {
                // Push `sql` since it's intact.
                if let Some(stat) = self.buf.as_mut() {
                    stat.push_sql(sql)?;
                } else {
                    let mut stat = Statement::new_empty();
                    stat.push_sql(sql)?;
                    self.inner.push(stat);
                }
            },
            _ => {},
        }

        Ok(())
    }

    /// End a statement and add it to the collection.
    ///
    /// This should only be used as an alternative to the special annotations
    /// `tern:begin` and `tern:end`.
    ///
    /// # Errors
    ///
    /// This returns an error if there is no statement open.
    pub fn end(&mut self) -> TernResult<()> {
        let Some(statement) = self.buf.take() else {
            return Err(TernError::QueryBuilder(
                "called `end` with no open statement",
            ))?;
        };
        self.inner.push(statement);
        Ok(())
    }

    /// Returns whether there is a statement that has not been closed yet.
    pub fn has_open_statement(&self) -> bool {
        self.buf.is_some()
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

fn no_tx_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".*(tern:noTransaction),?([a-z]*)").unwrap())
}

/// Any tern annotations from the first line of a SQL file/query.
#[derive(Debug, Default, Clone, Copy)]
struct Annotation {
    no_tx: bool,
    dialect: Option<SqlDialect>,
}

impl Annotation {
    fn new(header: &str) -> Self {
        let re = no_tx_re();
        let Some(caps) = re.captures(header) else {
            return Self::default();
        };
        let no_tx = caps.get(1).is_some();
        let dialect = caps.get(2).and_then(|m| {
            Some(match m.as_str() {
                "sqlite" => SqlDialect::Sqlite,
                "mysql" => SqlDialect::MySql,
                "postgres" => SqlDialect::Postgres,
                _ => return None,
            })
        });
        Self { no_tx, dialect }
    }

    fn in_tx(&self) -> bool {
        !self.no_tx
    }
}
