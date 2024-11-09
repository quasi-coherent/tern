use super::source::{MigrationQuery, MigrationSource};
use crate::error::Error;

use base64::{engine::general_purpose::STANDARD, Engine as _};
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
        let statements = query.statements()?;

        Ok(Self {
            version: source.version,
            description: Cow::Owned(source.description.to_string()),
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
            content: STANDARD.encode(self.sql.as_ref()),
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
    /// The base64 encoding of the SQL
    /// that ran when it was applied.
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
