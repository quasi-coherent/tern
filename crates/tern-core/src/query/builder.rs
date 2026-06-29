use regex::Regex;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead as _, BufReader, Cursor, Lines, Read};
use std::path::Path;
use std::str::FromStr as _;
use std::sync::OnceLock;

use crate::error::{TernError, TernResult};
use crate::query::split::{ReadQuery, SqlDialect};
use crate::query::{Query, Statement};

/// `QueryReader` reads a `Query` from some source.
pub struct QueryReader<R = ()> {
    reader: ReadQuery<R>,
    no_tx: bool,
}

impl QueryReader {
    /// Build a `Query` from the contents of a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> TernResult<QueryReader<File>> {
        let mut lines = Self::read_lines(&path)?;
        let Some(sql) = lines.next().transpose()? else {
            return Err(BuilderError::Empty)?;
        };
        let header = Header::from_line(&sql)?;
        let dialect = header.and_then(|h| h.dialect);
        let no_tx = header.is_some_and(|h| h.no_tx);
        let f = File::open(path)?;
        let reader = ReadQuery::new(f, dialect);
        Ok(QueryReader { reader, no_tx })
    }

    /// Build a `Query` from a raw SQL string.
    pub fn from_sql<T: ?Sized + AsRef<str>>(
        sql: &T,
    ) -> TernResult<QueryReader<&[u8]>> {
        let sql_str = sql.as_ref();
        let Some(l) = sql_str.lines().next() else {
            return Err(BuilderError::Empty)?;
        };
        let header = Header::from_line(l)?;
        let dialect = header.and_then(|h| h.dialect);
        let no_tx = header.is_some_and(|h| h.no_tx);
        Ok(QueryReader {
            reader: ReadQuery::new(sql_str.as_bytes(), dialect),
            no_tx,
        })
    }

    /// Builder from a generic `Read` with default settings.
    pub fn from_reader<R: Read>(read: R) -> QueryReader<R> {
        QueryReader { reader: ReadQuery::new(read, None), no_tx: false }
    }

    fn read_lines<P: AsRef<Path>>(
        path: P,
    ) -> TernResult<Lines<BufReader<File>>> {
        let f = File::open(path)?;
        Ok(BufReader::new(f).lines())
    }
}

impl<R: Read> QueryReader<R> {
    /// Build this for a `Query` to run outside a transaction.
    pub fn with_notx(self) -> Self {
        Self { no_tx: true, ..self }
    }

    /// Set a hint for the SQL dialect in use.
    ///
    /// Use if dialect-specific syntax is causing a misinterpretation of
    /// statement terminator characters.
    pub fn set_dialect(self, dialect: SqlDialect) -> Self {
        let inner = self.reader.into_inner();
        Self { reader: ReadQuery::new(inner, Some(dialect)), ..self }
    }

    /// Assemble a `Query` from this builder configuration.
    pub fn read_into(mut self) -> TernResult<Query> {
        let mut inner = BTreeSet::new();
        if self.no_tx {
            self.build_notx(&mut inner)?;
        } else {
            self.build_tx(&mut inner)?;
        }
        Ok(Query::new(inner, self.no_tx))
    }

    fn build_tx(&mut self, buf: &mut BTreeSet<Statement>) -> TernResult<()> {
        let mut idx = 0_u32;
        while let Some(sql_str) = self.reader.read_string()? {
            let stat = Statement(idx, sql_str);
            buf.insert(stat);
            idx += 1;
        }
        Ok(())
    }

    fn build_notx(&mut self, buf: &mut BTreeSet<Statement>) -> TernResult<()> {
        let mut stat = Statement::init();
        let mut in_tx = false;
        while let Some(sql_str) = self.reader.read_string()? {
            let annot = Stat::from_sql(&sql_str);
            if annot.is_some_and(Stat::begin) {
                if in_tx {
                    return Err(BuilderError::TernNote(format!(
                        "tern:begin_tx with existing open {sql_str}"
                    )))?;
                }
                stat.push_sql(sql_str);
                in_tx = true;
                // Skip pushing the current statement.
                continue;
            } else if annot.is_some_and(Stat::end) {
                if !in_tx {
                    return Err(BuilderError::TernNote(format!(
                        "tern:end_tx with none open {sql_str}"
                    )))?;
                }
                stat.push_sql(sql_str);
                in_tx = false;
            } else {
                stat.push_sql(sql_str);
            }

            // If we got here it's the end of a previous transaction or it's the
            // beginning and end of one.
            let new_stat = stat.take();
            buf.insert(new_stat);
        }
        if !stat.1.is_empty() {
            return Err(BuilderError::Eof(stat.1))?;
        }
        Ok(())
    }
}

/// `QueryBuilder` creates a `Query` by accumulating SQL.
#[derive(Clone, Debug, Default)]
pub struct QueryBuilder(String);

impl QueryBuilder {
    /// Push the raw SQL to the query builder.
    pub fn push_sql<T: AsRef<str>>(&mut self, sql: T) {
        self.0.push_str(sql.as_ref());
    }

    /// Finish pushing SQL to this builder, returning a `QueryReader` to read
    /// into a `Query` with [`QueryReader::read_into`].
    pub fn into_reader(self) -> QueryReader<Cursor<Vec<u8>>> {
        let read = self.0.into_bytes();
        let cur = Cursor::new(read);
        QueryReader::from_reader(cur)
    }
}

impl<R: Default + Read> Default for QueryReader<R> {
    fn default() -> Self {
        Self { reader: Default::default(), no_tx: false }
    }
}

// Local error type to write less.
#[derive(Debug, thiserror::Error)]
enum BuilderError {
    #[error("found empty query source")]
    Empty,
    #[error("invalid annotation: {0}")]
    TernNote(String),
    #[error("EOF with open transaction: {0}")]
    Eof(String),
}

impl From<BuilderError> for TernError {
    fn from(v: BuilderError) -> Self {
        Self::QueryBuilder(v.to_string())
    }
}

fn tern_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\-{2,}.*tern:([\w\,]*)").unwrap())
}

fn tern_annot(line: &str) -> Option<&str> {
    let re = tern_re();
    let caps = re.captures(line)?;
    let annot = caps.get(1)?;
    if annot.is_empty() {
        return None;
    }
    Some(annot.as_str())
}

// The acceptable ways to annotate the first line of a migration source.
#[derive(Clone, Copy, Debug, Default)]
struct Header {
    no_tx: bool,
    dialect: Option<SqlDialect>,
}

impl Header {
    fn from_line(line: &str) -> TernResult<Option<Self>> {
        let Some(annot) = tern_annot(line) else {
            return Ok(None);
        };
        let no_tx = annot.contains("noTransaction");
        // For both "noTransaction,pg" and "pg,noTransaction":
        let dstr = annot.replace("noTransaction", "").replace(",", "");
        if dstr.is_empty() {
            return Ok(Some(Self { no_tx, dialect: None }));
        }
        let dialect = SqlDialect::from_str(&dstr)?;
        Ok(Some(Self { no_tx, dialect: Some(dialect) }))
    }
}

// Acceptable ways to annotate SQL for beginning/ending a statement transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Stat {
    BeginTx,
    EndTx,
}

impl Stat {
    fn from_sql(sql: &str) -> Option<Self> {
        let ls = sql.lines();
        for line in ls {
            match tern_annot(line) {
                Some("tern:begin_tx") => return Some(Self::BeginTx),
                Some("tern:end_tx") => return Some(Self::EndTx),
                _ => continue,
            }
        }
        None
    }

    fn begin(self) -> bool {
        matches!(self, Self::BeginTx)
    }

    fn end(self) -> bool {
        !self.begin()
    }
}
