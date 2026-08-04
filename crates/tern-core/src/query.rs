//! Migration queries.
use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};
use std::path::Path;
use std::str::FromStr;

use crate::error::{TernError, TernResult};
use crate::internal::{Header, QueryReader};

// Placeholder for queries that don't exist yet.
const PENDING_QUERY: &str = "-- Query pending resolution.\nSELECT 1;";

/// The variant of SQL syntax contained in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqlDialect {
    /// MySQL or MariaDB.
    MySql,
    /// PostgreSQL.
    Postgres,
    /// SQLite.
    Sqlite,
}

impl FromStr for SqlDialect {
    type Err = TernError;

    fn from_str(s: &str) -> TernResult<Self> {
        Ok(match s.to_lowercase().trim() {
            "mysql" | "mariadb" => Self::MySql,
            "postgres" | "postgresql" | "pg" | "pgsql" => Self::Postgres,
            "sqlite" | "sqlite3" => Self::Sqlite,
            _ => {
                return Err(TernError::QueryBuilder(format!(
                    "unrecognized sql dialect {s}"
                )));
            },
        })
    }
}

impl Display for SqlDialect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::MySql => "mysql",
            Self::Postgres => "postgresql",
            Self::Sqlite => "sqlite",
        };
        f.write_str(s)
    }
}

/// A migration's SQL query.
#[derive(Clone, Debug)]
pub struct Query {
    pub(crate) inner: BTreeSet<Statement>,
    pub(crate) no_tx: bool,
}

impl Query {
    /// Returns a builder that can build a `Query` incrementally.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use tern_core::query::{Query, QueryBuilder};
    /// let mut b1 = Query::builder();
    /// b1.push_sql("CREATE TABLE blah (c1 text);");
    /// b1.push_sql("CREATE INDEX blah_idx ON blah (c1);");
    /// let q1 = b1.build().unwrap();
    ///
    /// // This builder built a query that runs in a transaction.
    /// assert_eq!(q1.size(), 1);
    ///
    /// let mut b2 = Query::builder().transaction(false);
    /// b2.push_sql("CREATE TABLE blah (c1 text);");
    /// b2.push_sql("CREATE INDEX blah_idx ON blah (c1);");
    ///
    /// // This builder built a query that has two statements running
    /// // independently outside of a transaction.
    /// let q2 = b2.build().unwrap();
    /// assert_eq!(q2.size(), 2);
    /// ```
    pub fn builder() -> QueryBuilder {
        QueryBuilder::default()
    }

    /// Placeholder for a query that resolves at a later time.
    pub fn pending() -> Self {
        let stat = Statement::new(0, PENDING_QUERY);
        let mut inner = BTreeSet::new();
        inner.insert(stat);
        Self { inner, no_tx: false }
    }

    /// Build the `Query` from a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> TernResult<Self> {
        let mut rdr = QueryReader::from_file(path)?;
        let mut inner = BTreeSet::new();
        rdr.build(&mut inner);
        Ok(Self { inner, no_tx: rdr.no_tx() })
    }

    /// Build the `Query` from raw SQL.
    pub fn from_sql<T: ?Sized + AsRef<str>>(sql: &T) -> TernResult<Self> {
        let mut rdr = QueryReader::from_sql(sql, None)?;
        let mut inner = BTreeSet::new();
        rdr.build(&mut inner);
        Ok(Self { inner, no_tx: rdr.no_tx() })
    }

    /// Set whether this `Query` should run in a transaction.
    pub fn transaction(mut self, yes: bool) -> Query {
        self.no_tx = !yes;
        self
    }

    /// Return the number of `Statement`s in this query.
    pub fn size(&self) -> usize {
        if !self.no_tx {
            return 1;
        }
        self.inner.len()
    }

    /// Returns whether this `Query` is to run in a database transaction.
    pub fn in_tx(&self) -> bool {
        !self.no_tx
    }

    /// Return the raw query in a string.
    pub fn into_raw(&self) -> String {
        let mut buf = String::new();
        let n = self.size();

        if self.no_tx {
            self.inner.iter().for_each(|st| {
                let num = st.idx;
                let q = &st.sql;
                buf.push_str(&format!("-- [Statement {num}/{n}]"));
                buf.push('\n');
                buf.push_str(q);
            })
        } else {
            let sql = self
                .inner
                .iter()
                .map(|st| st.raw())
                .collect::<Vec<_>>()
                .join("\n");
            buf.push_str(&sql);
        }

        buf
    }
}

impl Display for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.into_raw())
    }
}

/// Build a query incrementally.
#[derive(Clone, Debug, Default)]
pub struct QueryBuilder {
    buf: String,
    dialect: Option<SqlDialect>,
    no_tx: bool,
}

impl QueryBuilder {
    /// New empty `QueryBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder that provides a hint as to the dialect of SQL.
    pub fn with_dialect(mut self, dialect: SqlDialect) -> Self {
        self.dialect = Some(dialect);
        self
    }

    /// Set this to build a query that doesn't run in a transaction by default.
    pub fn no_transaction(mut self) -> Self {
        self.no_tx = true;
        self
    }

    /// Push the raw SQL to the query builder.
    pub fn push_sql<T: AsRef<str>>(&mut self, sql: T) {
        let sql = sql.as_ref().trim();
        if !sql.is_empty() {
            self.buf.push_str(sql);
            self.buf.push('\n');
        }
    }

    /// Finish pushing SQL to this builder, returning a `QueryReader` that can
    /// be to read into a `Query` with [`QueryReader::read_query`].
    pub fn build(self) -> TernResult<Query> {
        let h = self.header();
        let mut rdr = QueryReader::from_sql(&self.buf, Some(h))?;
        let mut inner = BTreeSet::new();
        rdr.build(&mut inner);
        Ok(Query { inner, no_tx: rdr.no_tx() })
    }

    fn header(&self) -> Header {
        Header { no_tx: self.no_tx, dialect: self.dialect }
    }
}

/// A SQL statement.
///
/// A `Statement` is one or more individual SQL expressions that are sent in one
/// prepared statement.
#[derive(Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub(crate) struct Statement {
    pub(crate) idx: u32,
    pub(crate) sql: String,
}

impl Statement {
    pub(crate) fn new<T: AsRef<str>>(idx: u32, sql: T) -> Self {
        Self { idx, sql: sql.as_ref().into() }
    }

    pub(crate) fn raw(&self) -> &str {
        &self.sql
    }

    pub(crate) fn push_sql<T: AsRef<str>>(&mut self, sql: T) {
        let v = sql.as_ref().trim();
        if !v.is_empty() {
            if !self.sql.is_empty() {
                self.sql.push('\n');
            }
            self.sql.push_str(v);
        }
    }
}
