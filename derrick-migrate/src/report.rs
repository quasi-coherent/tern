use base64::{engine::general_purpose::STANDARD, Engine as _};
use derrick_core::types::{AppliedMigration, Migration};
use log::{log, Level};

#[derive(Debug, Clone)]
pub struct MigrationReport {
    report: Vec<DisplayMigration>,
}

impl MigrationReport {
    pub fn new(report: Vec<DisplayMigration>) -> Self {
        Self { report }
    }

    pub fn display_report(&self) {
        for m in self.report.iter() {
            m.display_migration(Level::Info)
        }
    }
}

#[derive(Debug, Clone)]
pub struct DisplayMigration {
    version: i64,
    state: MigrationState,
    description: String,
    sql: String,
    transactional: Transactional,
    duration_ms: RunDuration,
    error_reason: MigrationErrors,
}

impl DisplayMigration {
    pub fn display_migration(&self, level: Level) {
        log!(level, "{:#?}", self)
    }

    pub fn from_unapplied(value: &Migration) -> Self {
        Self {
            version: value.version,
            state: MigrationState::Unapplied,
            description: value.description.to_string(),
            sql: Self::preview_sql(&value.sql),
            transactional: Transactional::from_boolean(value.no_tx),
            duration_ms: RunDuration::Unapplied,
            error_reason: MigrationErrors::None,
        }
    }

    pub fn from_failed(value: &Migration, reason: String) -> Self {
        Self {
            version: value.version,
            state: MigrationState::FailedUnapplied,
            description: value.description.to_string(),
            sql: Self::preview_sql(&value.sql),
            transactional: Transactional::from_boolean(value.no_tx),
            duration_ms: RunDuration::Unapplied,
            error_reason: MigrationErrors::Reason(reason),
        }
    }

    pub fn from_existing(value: &AppliedMigration) -> Self {
        Self::from_applied(value, MigrationState::Existing)
    }

    pub fn from_new_applied(value: &AppliedMigration) -> Self {
        Self::from_applied(value, MigrationState::NewApplied)
    }

    fn from_applied(value: &AppliedMigration, state: MigrationState) -> Self {
        let sql = match STANDARD.decode(&value.content.as_bytes()) {
            Ok(b) => {
                if let Ok(decoded) = std::str::from_utf8(&b) {
                    decoded.to_string()
                } else {
                    format!("error base64 decoding...{}", value.content)
                }
            }
            _ => value.content.to_string(),
        };

        Self {
            state,
            version: value.version,
            description: value.description.to_string(),
            sql: Self::preview_sql(&sql),
            transactional: Transactional::NotApplicable,
            duration_ms: RunDuration::Duration(value.duration_ms),
            error_reason: MigrationErrors::None,
        }
    }

    fn preview_sql(sql: &str) -> String {
        let res = sql.lines().take(10).collect::<Vec<_>>().join("\n") + "...";
        res.to_string()
    }
}

#[derive(Debug, Clone, Copy)]
enum MigrationState {
    Existing,
    Unapplied,
    FailedUnapplied,
    NewApplied,
}

impl std::fmt::Display for MigrationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Existing => write!(f, "EXISTING"),
            Self::Unapplied => write!(f, "UNAPPLIED"),
            Self::FailedUnapplied => write!(f, "FAIL"),
            Self::NewApplied => write!(f, "NEW_APPLIED"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Transactional {
    NoTransaction,
    InTransaction,
    NotApplicable,
}

impl std::fmt::Display for Transactional {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoTransaction => write!(f, "NO_TRANSACTION"),
            Self::InTransaction => write!(f, "IN_TRANSACTION"),
            Self::NotApplicable => write!(f, "N/A"),
        }
    }
}

impl Transactional {
    fn from_boolean(v: bool) -> Self {
        if v {
            return Self::NoTransaction;
        };
        Self::InTransaction
    }
}

#[derive(Debug, Clone, Copy)]
enum RunDuration {
    Duration(i64),
    Unapplied,
}

impl std::fmt::Display for RunDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duration(ms) => write!(f, "{}ms", ms),
            Self::Unapplied => write!(f, "UNAPPLIED"),
        }
    }
}

#[derive(Debug, Clone)]
enum MigrationErrors {
    Reason(String),
    None,
}

impl std::fmt::Display for MigrationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reason(e) => write!(f, "{}", e),
            Self::None => write!(f, "SUCCESS"),
        }
    }
}
