use super::source::{MigrationQuery, MigrationSource};
use crate::error::Error;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use sqlparser::{
    dialect::GenericDialect,
    parser::{Parser, ParserOptions},
};
use std::borrow::Cow;

/// A migration that can be applied.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration version.
    pub version: i64,
    /// Migration description.
    pub description: Cow<'static, str>,
    /// The file content of the migration.
    pub content: Cow<'static, str>,
    /// The query to run.
    pub sql: Cow<'static, str>,
    /// The query divided into its individual statements.
    pub statements: Cow<'static, [String]>,
    /// If true, not ran in a transaction.
    pub no_tx: bool,
}

impl Migration {
    pub fn new(source: &MigrationSource, query: MigrationQuery) -> Result<Self, Error> {
        let sql = query.sql();
        let no_tx = query.no_tx();
        let opts = ParserOptions::new()
            .with_trailing_commas(true)
            .with_unescape(false);
        let dialect = GenericDialect {};
        let statements = Parser::new(&dialect)
            .with_options(opts)
            .try_with_sql(sql)?
            .parse_statements()?
            .into_iter()
            .map(|statement| statement.to_string())
            .collect::<Vec<_>>();

        Ok(Self {
            version: source.version,
            description: Cow::Owned(source.description.replace("_", " ")),
            content: Cow::Owned(source.content.to_string()),
            sql: Cow::Owned(sql.to_string()),
            statements: Cow::Owned(statements),
            no_tx,
        })
    }

    pub fn new_applied(&self, duration_ms: i64) -> AppliedMigration {
        AppliedMigration {
            duration_ms,
            description: self.description.to_string(),
            content: STANDARD.encode(self.content.as_ref()),
            version: self.version,
        }
    }
}

/// A migration that was just applied.
#[derive(Debug, Clone)]
pub struct AppliedMigration {
    /// Migration version.
    pub version: i64,
    /// Migration description.
    pub description: String,
    /// The base64 encoding of the migration file
    /// when it was applied.
    pub content: String,
    /// Apply duration.
    pub duration_ms: i64,
}

impl AppliedMigration {
    pub fn order_by_asc(mut applied: Vec<AppliedMigration>) -> Vec<AppliedMigration> {
        applied.sort_by_key(|m| m.version);
        applied
    }
}
