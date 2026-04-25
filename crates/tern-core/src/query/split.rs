// This comes directly from the crate https://github.com/HelgeSverre/sql-splitter
// namely the module `parser` except with almost all of it deleted.  Only using
// it for the task of splitting a plain sql file on ";" while hopefully handling
// most of the weird edge cases.
//
// But `sql-splitter` is really for something else completely, it's for doing
// this but for, like, 50GB of SQL.
//
// The crate itself has no feature flags[1] to avoid the huge dependency
// footprint that exists to solve whatever that problem is: a ton of crates in
// scope for decompressing in all the ways something can be compressed, bindings
// to duckdb, which builds and links with libduckdb-sys, it brings in arrow in
// its entirety.  All that totaled leads to this crate taking 45 minutes to
// build in CI versus 3 minutes if I copy/paste the below.
//
// [1]: https://github.com/HelgeSverre/sql-splitter/issues/40
use std::io::{BufRead, BufReader, Error as IoError, Read};
use std::str::FromStr;

use crate::error::{TernError, TernResult};

// Default capacity of `std::io::BufReader`.
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

/// The variant of SQL syntax contained in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SqlDialect {
    MySql,
    #[default]
    Postgres,
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

pub(super) struct ReadQuery<R> {
    reader: BufReader<R>,
    stmt_buffer: Vec<u8>,
    dialect: Option<SqlDialect>,
}

impl<R: Read> ReadQuery<R> {
    pub(super) fn new(reader: R, dialect: Option<SqlDialect>) -> Self {
        Self {
            reader: BufReader::with_capacity(DEFAULT_BUF_SIZE, reader),
            stmt_buffer: Vec::with_capacity(32 * 1024),
            dialect,
        }
    }

    fn is_mysql(&self) -> bool {
        self.dialect.is_some_and(|d| d == SqlDialect::MySql)
    }

    fn is_pg(&self) -> bool {
        self.dialect.is_some_and(|d| d == SqlDialect::Postgres)
    }

    pub(super) fn into_inner(self) -> R {
        self.reader.into_inner()
    }

    pub(super) fn read_string(&mut self) -> TernResult<Option<String>> {
        self.read_bytes()
            .and_then(|b| {
                b.map(String::from_utf8).transpose().map_err(IoError::other)
            })
            .map_err(TernError::from)
    }

    fn read_bytes(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        let is_psql = self.is_pg();
        let is_mysql = self.is_mysql();

        self.stmt_buffer.clear();
        let mut inside_single_quote = false;
        let mut inside_double_quote = false;
        let mut escaped = false;
        let mut in_line_comment = false;
        let mut in_block_comment = false;

        // For PostgreSQL dollar-quoting: track the tag
        let mut in_dollar_quote = false;
        let mut dollar_tag: Vec<u8> = Vec::new();

        loop {
            let buf = self.reader.fill_buf()?;
            if buf.is_empty() {
                if self.stmt_buffer.is_empty() {
                    return Ok(None);
                }
                let result = std::mem::take(&mut self.stmt_buffer);
                return Ok(Some(result));
            }
            let mut consumed = 0;
            let mut found_terminator = false;

            for (i, &b) in buf.iter().enumerate() {
                let inside_string = inside_single_quote
                    || inside_double_quote
                    || in_dollar_quote;

                let buf_len = buf.len();
                // End of line comment on newline
                if in_line_comment {
                    if b == b'\n' {
                        in_line_comment = false;
                    }
                    continue;
                }

                // Skip bytes inside a block comment and close on `*/`.
                if in_block_comment {
                    if b == b'*' && i + 1 < buf_len && buf[i + 1] == b'/' {
                        in_block_comment = false;
                    }
                    continue;
                }

                if escaped {
                    escaped = false;
                    continue;
                }

                // Handle backslash escapes (MySQL style)
                if b == b'\\' && inside_string && is_mysql {
                    escaped = true;
                    continue;
                }

                // Handle block comments (/* ... */)
                if b == b'/'
                    && !inside_string
                    && i + 1 < buf_len
                    && buf[i + 1] == b'*'
                {
                    in_block_comment = true;
                    continue;
                }

                // Handle line comments (-- to end of line)
                if b == b'-'
                    && !inside_string
                    && i + 1 < buf_len
                    && buf[i + 1] == b'-'
                {
                    in_line_comment = true;
                    continue;
                }

                // Handle '#' line comments in MySQL
                if b == b'#' && !inside_string && i + 1 < buf_len && is_mysql {
                    in_line_comment = true;
                    continue;
                }

                // Handle dollar-quoting for PostgreSQL
                if is_psql && !inside_single_quote && !inside_double_quote {
                    if b == b'$' && !in_dollar_quote {
                        // Start of dollar-quote: scan for the closing $
                        if let Some(end) =
                            buf[i + 1..].iter().position(|&c| c == b'$')
                        {
                            let tag_bytes = &buf[i + 1..i + 1 + end];
                            // Validate tag: must be empty OR identifier-like
                            // [A-Za-z_][A-Za-z0-9_]*
                            let is_valid_tag = if tag_bytes.is_empty() {
                                true
                            } else {
                                let mut iter = tag_bytes.iter();
                                match iter.next() {
                                    Some(&first)
                                        if first.is_ascii_alphabetic()
                                            || first == b'_' =>
                                    {
                                        iter.all(|&c| {
                                            c.is_ascii_alphanumeric()
                                                || c == b'_'
                                        })
                                    },
                                    _ => false,
                                }
                            };

                            if is_valid_tag {
                                dollar_tag = tag_bytes.to_vec();
                                in_dollar_quote = true;
                                continue;
                            }
                            // Invalid tag - treat $ as normal character
                        }
                    } else if b == b'$' && in_dollar_quote {
                        // Potential end of dollar-quote
                        let tag_len = dollar_tag.len();
                        if i + 1 + tag_len < buf.len()
                            && buf[i + 1..i + 1 + tag_len] == dollar_tag[..]
                            && buf.get(i + 1 + tag_len) == Some(&b'$')
                        {
                            in_dollar_quote = false;
                            dollar_tag.clear();
                            continue;
                        }
                    }
                }

                if b == b'\'' && !inside_double_quote && !in_dollar_quote {
                    inside_single_quote = !inside_single_quote;
                } else if b == b'"' && !inside_single_quote && !in_dollar_quote
                {
                    inside_double_quote = !inside_double_quote;
                } else if b == b';' && !inside_string {
                    self.stmt_buffer.extend_from_slice(&buf[..=i]);
                    consumed = i + 1;
                    found_terminator = true;
                    break;
                }
            }

            if found_terminator {
                self.reader.consume(consumed);
                let result = std::mem::take(&mut self.stmt_buffer);
                return Ok(Some(result));
            }

            self.stmt_buffer.extend_from_slice(buf);
            let len = buf.len();
            self.reader.consume(len);
        }
    }
}

impl<R: Read + Default> Default for ReadQuery<R> {
    fn default() -> Self {
        ReadQuery::new(R::default(), None)
    }
}
