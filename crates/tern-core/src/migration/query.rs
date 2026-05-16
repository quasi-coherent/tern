//! Defining the query for a migration.
use regex::Regex;
use std::fmt::{self, Display, Formatter, Write as _};
use std::io::{BufRead as _, BufReader, Error as IoError, Read};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::OnceLock;
use std::vec::IntoIter;

use crate::error::{TernError, TernResult};
use crate::migration::split::{Parser, SqlDialect};

// Default capacity of `std::io::BufReader`.
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

/// `Query` holds the SQL statements to run when applying a migration.
///
/// Build one using one of the builder types.  The [`builder`](Query::builder)
/// method builds a query that is sent as one prepared statement.
///
/// Contrast with [`sequential_builder`](Query::sequential_builder), which can
/// be used to build a migration query that runs groups of statements
/// sequentially, exiting on the first error.
///
/// It is important to recognize that this needs to be a full statement.  If it
/// lacks a closing terminator ';', one is added, so in particular, building a
/// `Query` from fragments of SQL is not possible.  If this needs to be done,
/// assemble the fragments into one statement and push that to the builder
/// instead.
#[derive(Clone, Debug)]
pub enum Query {
    /// A `Statement` with one or more individual expressions to run in a
    /// transaction.
    Tx(Statement),
    /// `Statement`s to run sequentially outside of a transaction, exiting on
    /// the first error.
    Seq(Vec<Statement>),
}

impl Query {
    /// Return a default `TxBuilder` to build a `Query::Tx`.
    ///
    /// This is mostly used in building queries for Rust migrations.
    pub fn builder() -> TxBuilder {
        TxBuilder::default()
    }

    /// Return a default `SeqBuilder` to build a `Query::Seq`.
    ///
    /// This is primarily used in building queries for Rust migrations.
    pub fn sequential_builder() -> SeqBuilder {
        SeqBuilder::default()
    }

    /// Create a `Query` from a path to a SQL file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> TernResult<Self> {
        let f = std::fs::File::open(path)?;

        let mut reader = BufReader::new(f);
        let mut sql = String::new();
        let _ = reader.read_line(&mut sql)?;
        let note = Annotation::new(&sql);

        if note.in_tx() {
            let mut f = reader.into_inner();
            let _ = f.read_to_string(&mut sql)?;
            let mut tx = TxBuilder::default();
            tx.push_sql(sql)?;
            Ok(tx.build())
        } else {
            Self::as_seq(reader, note.dialect)
        }
    }

    /// Create a `Query` directly from a SQL string.
    pub fn from_sql<T: AsRef<str>>(sql: T) -> TernResult<Self> {
        let sql = sql.as_ref();

        let mut lines = sql.trim().lines();
        let Some(header) = lines.next() else {
            return Err(BuilderError::Empty)?;
        };
        let note = Annotation::new(header);

        if note.in_tx() {
            let mut tx = TxBuilder::default();
            tx.push_sql(sql)?;
            Ok(tx.build())
        } else {
            Self::as_seq(sql.as_bytes(), note.dialect)
        }
    }

    /// Return whether this query is in a transaction.
    pub fn in_tx(&self) -> bool {
        matches!(self, Query::Tx(_))
    }

    /// Return the statement held by [`Query::Tx`] or `None` if the value is
    /// not that variant.
    pub fn get_tx(&self) -> Option<&Statement> {
        let Self::Tx(s) = self else {
            return None;
        };
        Some(s)
    }

    /// Returns the number of statements in this `Query`.
    pub fn size(&self) -> usize {
        let Self::Seq(q) = self else {
            return 1;
        };
        q.len()
    }

    fn as_seq<R: Read>(
        reader: R,
        dialect: Option<SqlDialect>,
    ) -> TernResult<Self> {
        let d = dialect.unwrap_or(SqlDialect::Postgres);
        let mut parser = Parser::with_dialect(reader, DEFAULT_BUF_SIZE, d);
        let mut seq = SeqBuilder::default();

        while let Some(stat_bytes) = parser.read_statement()? {
            let sql = String::from_utf8(stat_bytes).map_err(IoError::other)?;
            seq.push_sql(sql)?;
        }

        Ok(seq.build())
    }
}

impl Display for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tx(s) => s.fmt(f),
            Self::Seq(ss) => ss
                .iter()
                .map(Deref::deref)
                .collect::<Vec<_>>()
                .join("\n\n")
                .fmt(f),
        }
    }
}

impl FromIterator<Statement> for Query {
    fn from_iter<T: IntoIterator<Item = Statement>>(iter: T) -> Self {
        Self::Seq(iter.into_iter().collect())
    }
}

impl IntoIterator for Query {
    type Item = Statement;
    type IntoIter = StatementIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Tx(s) => StatementIter::from_statement(s),
            Self::Seq(ss) => StatementIter::new(ss),
        }
    }
}

/// `Iterator` for `Query`.
#[derive(Debug, Clone, Default)]
pub struct StatementIter {
    inner: IntoIter<Statement>,
}

impl StatementIter {
    fn new(inner: Vec<Statement>) -> Self {
        Self { inner: inner.into_iter() }
    }

    fn from_statement(stat: Statement) -> Self {
        let inner = vec![stat];
        Self::new(inner)
    }
}

impl Iterator for StatementIter {
    type Item = Statement;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

// Local error type to write less.
#[derive(Debug, thiserror::Error)]
enum BuilderError {
    #[error("invalid for builder with open statement")]
    AlreadyOpen,
    #[error("invalid for builder with no open statement")]
    NoOpen,
    #[error("found empty query source")]
    Empty,
}

impl From<BuilderError> for TernError {
    fn from(v: BuilderError) -> Self {
        Self::QueryBuilder(v.to_string())
    }
}

/// `TxBuilder` builds a [`Query::Tx`].
///
/// This is intended to be used when building a query for a Rust migration.
#[derive(Debug, Clone, Default)]
pub struct TxBuilder {
    buf: Statement,
}

impl TxBuilder {
    /// Push a valid SQL statement to this builder.
    ///
    /// This does nothing if the input is empty or consists only of whitespace.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the active buffer fails.
    pub fn push_sql<T: AsRef<str>>(&mut self, sql: T) -> TernResult<()> {
        let sql_str = sql.as_ref().trim();

        // Ensure the input ends with ";".
        let stat = if sql_str.is_empty() {
            return Ok(());
        } else if sql_str.ends_with(";") {
            sql_str.to_string()
        } else {
            format!("{sql_str};")
        };
        writeln!(&mut self.buf.0, "{stat}").map_err(IoError::other)?;

        Ok(())
    }

    /// Consumes this value, returning it in a [`Query::Tx`].
    pub fn build(self) -> Query {
        Query::Tx(self.build_statement())
    }

    /// Consume this value, returning it as a `Statement`.
    pub fn build_statement(self) -> Statement {
        self.buf
    }
}

/// `SeqBuilder` builds a [`Query::Seq`].
///
/// This is intended to be used when building a query for a Rust migration.
#[derive(Debug, Clone, Default)]
pub struct SeqBuilder {
    tx: Option<TxBuilder>,
    stats: Vec<Statement>,
}

impl SeqBuilder {
    /// Start a new transaction.
    ///
    /// This should be used as an alternative to the special annotations
    /// `tern:begin_tx` and `tern:end_tx`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is already an open statement.
    pub fn begin_tx(&mut self) -> TernResult<()> {
        if self.tx.is_some() {
            Err(BuilderError::AlreadyOpen)?
        } else {
            self.tx = Some(TxBuilder::default());
            Ok(())
        }
    }

    /// Push a valid SQL statement to this builder.
    ///
    /// The first line is examined for the special markers `tern:begin_tx` and
    /// `tern:end_tx`, and a transaction is either started or ended if they are
    /// found and that marker is consistent with the state of this builder.
    ///
    /// If there is an open transaction, the input is added to it.  Otherwise it
    /// creates a statement of its own.
    ///
    /// # Errors
    ///
    /// An error is returned if the begin/end marker is found but this builder
    /// has/does not have an open transaction already.  This also results in an
    /// error if the value could not be written.
    pub fn push_sql<T: AsRef<str>>(&mut self, sql: T) -> TernResult<()> {
        let mut lines = sql.as_ref().trim().lines();

        match lines.next() {
            Some(l) if l.contains("tern:begin_tx") => {
                self.begin_tx()?;
                // We literally just put a value there.
                let tx = self.tx.as_mut().expect("impossible");
                let rest: &str = &lines.collect::<Vec<_>>().join("\n");
                tx.push_sql(rest)?;
            },
            Some(l) if l.contains("tern:end_tx") => {
                self.end_tx()?;
                let rest: &str = &lines.collect::<Vec<_>>().join("\n");
                // Recurse with the input minus first line.
                return self.push_sql(rest);
            },
            Some(_) => {
                // Push `sql` since it's not missing the first line.
                if let Some(tx) = self.tx.as_mut() {
                    tx.push_sql(sql)?;
                } else {
                    let mut tx = TxBuilder::default();
                    tx.push_sql(sql)?;
                    let stat = tx.build_statement();
                    self.stats.push(stat);
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
    pub fn end_tx(&mut self) -> TernResult<()> {
        let Some(tx) = self.tx.take() else {
            return Err(BuilderError::NoOpen)?;
        };
        let stat = tx.build_statement();
        self.stats.push(stat);
        Ok(())
    }

    /// Consume this value, returning a [`Query::Seq`] containing it.
    pub fn build(self) -> Query {
        Query::Seq(self.stats)
    }
}

/// `Statement` holds one query to be sent to the database.
///
/// This may be comprised of several individual expressions, in which case they
/// will be executed in a database transaction.
#[derive(Debug, Clone, Default)]
pub struct Statement(String);

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

impl Display for Statement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dialect() {
        let x = "-- tern:noTransaction";
        let y = "-- tern:noTransaction,sqlite";
        let z = "-- Comment with tern:noTransaction,postgres in the middle";
        let t = "-- tern:noTransaction,dynamodb";
        let xx = Annotation::new(x);
        let yy = Annotation::new(y);
        let zz = Annotation::new(z);
        let tt = Annotation::new(t);
        assert_eq!(xx.dialect, None);
        assert_eq!(yy.dialect, Some(SqlDialect::Sqlite));
        assert_eq!(zz.dialect, Some(SqlDialect::Postgres));
        assert_eq!(tt.dialect, None);
    }

    #[test]
    fn dollar_quoted_postgres() {
        // Semicolons inside $$ bodies must not split the statement.
        const SQL: &str = "
CREATE FUNCTION add(a int, b int) RETURNS int AS $$
BEGIN
  RETURN a + b; -- semicolon inside dollar body
END;
$$ LANGUAGE plpgsql;";
        let res = Query::from_sql(SQL);
        let q = res.unwrap();
        assert!(q.in_tx());
        assert_eq!(q.size(), 1);
    }

    #[test]
    fn dollar_quoted_with_tag_postgres() {
        const SQL: &str = "
-- tern:noTransaction,postgres
DO $body$
BEGIN
  RAISE NOTICE 'step; one';
END;
$body$;

SELECT 1;
";
        let res = Query::from_sql(SQL);
        assert!(res.is_ok_and(|q| q.size() == 2));
    }

    #[test]
    fn mysql_backslash_escape() {
        // Backslash-escaped quote inside a string must not end the string early.
        const SQL: &str = "
-- tern:noTransaction,mysql
INSERT INTO t (col) VALUES ('it\\'s fine');

SELECT 1;";
        let res = Query::from_sql(SQL);
        assert!(res.is_ok_and(|q| q.size() == 2));
    }

    #[test]
    fn handles_multiple() {
        const SQL: &str = r#"
-- tern:noTransaction,postgres
SELECT
  column1 AS "asdf;lkh",
  column2
FROM
  the_table as a
/* Why
would anyone do this;
it's absurd
*/
JOIN
  the_other_table as b
USING (column3);

SELECT * INTO the_table_recent
FROM the_table
WHERE
  column1 != 'string--with--special/*characters*/and--terminator;'
  AND recent = true;
"#;
        let res = Query::from_sql(SQL);
        assert!(res.is_ok_and(|q| q.size() == 2));
    }
}
