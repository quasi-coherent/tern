// This comes directly from the crate https://github.com/HelgeSverre/sql-splitter
// namely the module `parser` except with almost all of it deleted.  Only using
// it for the task of splitting a plain sql file on ";" while hopefully handling
// most of the weird edge cases.
//
// But `sql-splitter` is really for something else completely, it's for doing this
// but for, like, 50GB of SQL...
//
// The crate itself has no feature flags[1] to avoid the huge dependency
// footprint that exists to solve whatever that problem is: a ton of crates in
// scope for decompressing in all the ways something can be compressed, the rust
// bindings to the C++ analytics DB duckdb, which therefore must build and link
// and blah blah; it brings in all of arrow and seemingly every random add-on.
//
// All that totaled leads to this crate taking 45 minutes to build in CI.
//
// [1]: https://github.com/HelgeSverre/sql-splitter/issues/40
use std::io::{BufRead, BufReader, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum SqlDialect {
    MySql,
    #[default]
    Postgres,
    Sqlite,
}

impl std::str::FromStr for SqlDialect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mysql" | "mariadb" => Ok(SqlDialect::MySql),
            "postgres" | "postgresql" | "pg" => Ok(SqlDialect::Postgres),
            "sqlite" | "sqlite3" => Ok(SqlDialect::Sqlite),
            _ => Err(format!(
                "Unknown dialect: {}. Valid options: mysql, postgres, sqlite, mssql",
                s
            )),
        }
    }
}

pub(super) struct Parser<R: Read> {
    reader: BufReader<R>,
    stmt_buffer: Vec<u8>,
    dialect: SqlDialect,
}

impl<R: Read> Parser<R> {
    pub(super) fn with_dialect(
        reader: R,
        buffer_size: usize,
        dialect: SqlDialect,
    ) -> Self {
        Self {
            reader: BufReader::with_capacity(buffer_size, reader),
            stmt_buffer: Vec::with_capacity(32 * 1024),
            dialect,
        }
    }

    pub(super) fn read_statement(
        &mut self,
    ) -> std::io::Result<Option<Vec<u8>>> {
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

                // End of line comment on newline
                if in_line_comment {
                    if b == b'\n' {
                        in_line_comment = false;
                    }
                    continue;
                }

                // Skip bytes inside a block comment and close on `*/`.
                if in_block_comment {
                    if b == b'*' && i + 1 < buf.len() && buf[i + 1] == b'/' {
                        in_block_comment = false;
                    }
                    continue;
                }

                if escaped {
                    escaped = false;
                    continue;
                }

                // Handle backslash escapes (MySQL style)
                if b == b'\\'
                    && inside_string
                    && self.dialect == SqlDialect::MySql
                {
                    escaped = true;
                    continue;
                }

                // Handle block comments (/* ... */)
                if b == b'/'
                    && !inside_string
                    && i + 1 < buf.len()
                    && buf[i + 1] == b'*'
                {
                    in_block_comment = true;
                    continue;
                }

                // Handle line comments (-- to end of line)
                if b == b'-'
                    && !inside_string
                    && i + 1 < buf.len()
                    && buf[i + 1] == b'-'
                {
                    in_line_comment = true;
                    continue;
                }

                // Handle dollar-quoting for PostgreSQL
                if self.dialect == SqlDialect::Postgres
                    && !inside_single_quote
                    && !inside_double_quote
                {
                    if b == b'$' && !in_dollar_quote {
                        // Start of dollar-quote: scan for the closing $
                        if let Some(end) =
                            buf[i + 1..].iter().position(|&c| c == b'$')
                        {
                            let tag_bytes = &buf[i + 1..i + 1 + end];

                            // Validate tag: must be empty OR identifier-like [A-Za-z_][A-Za-z0-9_]*
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
