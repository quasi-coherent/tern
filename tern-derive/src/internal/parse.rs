use regex::Regex;
use std::path::{Path, PathBuf};
use std::{env, ffi::OsStr, fs, sync::OnceLock};

pub fn cargo_manifest_dir() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    PathBuf::from(manifest_dir)
}

fn filename_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^V(\d+)__(\w+)\.(sql|rs)$").unwrap())
}

#[derive(Debug, Clone)]
pub struct SqlSource {
    pub module: String,
    pub version: i64,
    pub description: String,
    pub content: String,
    pub no_tx: bool,
}

#[derive(Debug, Clone)]
pub struct RustSource {
    pub module: String,
    pub version: i64,
    pub description: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum MigrationSource {
    Sql(SqlSource),
    Rs(RustSource),
}

impl MigrationSource {
    pub fn from_migration_dir(
        migration_dir: impl AsRef<Path>,
    ) -> Result<Vec<MigrationSource>, SourceError> {
        let location = migration_dir.as_ref().canonicalize().map_err(|e| {
            SourceError::Path(
                format!(
                    "invalid migration path {:?}",
                    migration_dir.as_ref().to_path_buf()
                ),
                e.to_string(),
            )
        })?;
        let mut sources = Self::parse_sources(location)?;

        // order asc by version
        sources.sort_by_key(|s| match s {
            Self::Sql(s) => s.version,
            Self::Rs(s) => s.version,
        });
        Validator::new(sources.iter().map(|v| v.migration_id()).collect::<Vec<_>>()).validate()?;

        Ok(sources)
    }

    fn parse_sources(location: PathBuf) -> Result<Vec<MigrationSource>, SourceError> {
        let sources = fs::read_dir(location)
            .map_err(|_| SourceError::Directory("could not read migration directory".to_string()))?
            .filter_map(|entry| {
                let e = entry.ok()?;
                if e.file_name()
                    .to_str()
                    .is_some_and(|f| f == "mod.rs" || f.starts_with("."))
                {
                    None
                } else {
                    Some(e.path())
                }
            })
            .map(Self::parse)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sources)
    }

    fn migration_id(&self) -> (i64, String) {
        match self {
            Self::Sql(SqlSource {
                version,
                description,
                ..
            }) => (*version, description.clone()),
            Self::Rs(RustSource {
                version,
                description,
                ..
            }) => (*version, description.clone()),
        }
    }

    fn parse(filepath: impl AsRef<Path>) -> Result<Self, SourceError> {
        let filepath = filepath.as_ref();
        let module = filepath.file_stem().ok_or(SourceError::Name(format!(
            "no filename stem found {:?}",
            filepath.to_str()
        )))?;
        let (ver, description, ext) = filepath
            .file_name()
            .and_then(|n| {
                let filename = OsStr::to_str(n)?;
                let captures = filename_re().captures(filename)?;
                let version = captures.get(1)?.as_str();
                let description = captures.get(2)?.as_str();
                let source_type = captures.get(3)?.as_str();
                Some((version, description, source_type))
            })
            .ok_or(SourceError::Name(format!(
                r"format is `^V(\d+)__(\w+)\.(sql|rs)$`, got {:?}",
                filepath.to_str(),
            )))?;
        let version: i64 = ver
            .parse()
            .map_err(|_| SourceError::Name("invalid version, expected i64".to_string()))?;
        let source_type = SourceType::from_ext(ext)?;
        let content = fs::read_to_string(filepath).map_err(|e| SourceError::Io(e.to_string()))?;
        let module = module
            .to_str()
            .ok_or(SourceError::Name(
                "utf-8 decoding filename failed".to_string(),
            ))?
            .to_string();
        let this = match source_type {
            SourceType::Sql => {
                let no_tx = Self::no_tx(&content);
                let sql_source = SqlSource {
                    module,
                    version,
                    description: description.to_string(),
                    content,
                    no_tx,
                };
                Self::Sql(sql_source)
            }
            _ => {
                let rust_source = RustSource {
                    module,
                    version,
                    description: description.to_string(),
                    content,
                };
                Self::Rs(rust_source)
            }
        };

        Ok(this)
    }

    /// For static SQL migrations, parse the first line to see if the special
    /// `tern:noTransaction` annotation is present.
    fn no_tx(content: &str) -> bool {
        content
            .lines()
            .take(1)
            .next()
            .map(|l| l.contains("tern:noTransaction"))
            .unwrap_or_default()
    }
}

struct Validator {
    ids: Vec<(i64, String)>,
}

impl Validator {
    fn new(mut ids: Vec<(i64, String)>) -> Self {
        ids.sort_by_key(|(v, _)| *v);
        Self { ids }
    }

    fn duplicate_versions(&self) -> Result<(), SourceError> {
        let mut m = std::collections::HashMap::new();
        let mut offending_versions = Vec::new();
        for (version, description) in &self.ids {
            if m.insert(version, description).is_some() {
                offending_versions.push(*version);
            }
        }
        if !offending_versions.is_empty() {
            return Err(Version {
                message: "duplicate migration version found".to_string(),
                offending_versions,
            })?;
        }

        Ok(())
    }

    fn missing_versions(&self) -> Result<(), SourceError> {
        let size = self.ids.len() as i64;
        match self.ids.last() {
            Some((v, _)) if *v != size => {
                for (ix, (version, _)) in self.ids.iter().enumerate() {
                    let expected = (ix + 1) as i64;
                    if *version != expected {
                        return Err(Version {
                            message: format!(
                                "expected version {expected} for a set with {size} migrations"
                            ),
                            offending_versions: vec![*version],
                        })?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn validate(&self) -> Result<(), SourceError> {
        self.duplicate_versions()?;
        self.missing_versions()?;
        Ok(())
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum SourceError {
    Directory(String),
    Path(String, String),
    Name(String),
    Ext(String),
    Io(String),
    Sql(i64, String),
    Version(Version),
}

impl From<Version> for SourceError {
    fn from(value: Version) -> Self {
        Self::Version(value)
    }
}

pub struct Version {
    message: String,
    offending_versions: Vec<i64>,
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vs = self
            .offending_versions
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let versions_field = format!("[{vs}]");
        f.debug_struct("Version")
            .field("message", &self.message)
            .field("offending_versions", &versions_field)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
enum SourceType {
    Sql,
    Rust,
}

impl SourceType {
    pub fn from_ext(ext: &str) -> Result<Self, SourceError> {
        match ext {
            "sql" => Ok(Self::Sql),
            "rs" => Ok(Self::Rust),
            _ => Err(SourceError::Ext(format!(
                "got file extension {ext}, expected `sql` or `rs`"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceError, Validator, Version};

    fn to_validator(vs: Vec<i64>) -> Validator {
        let ids = vs
            .into_iter()
            .map(|v| (v, v.to_string()))
            .collect::<Vec<_>>();
        Validator::new(ids)
    }

    #[test]
    fn duplicate_version() {
        let vs = vec![1, 2, 3, 3, 4, 5, 6, 6, 7];
        let validator = to_validator(vs);
        let res = validator.duplicate_versions();
        assert!(
            matches!(res, Err(SourceError::Version(Version { offending_versions, ..})) if offending_versions == vec![3, 6])
        );
    }

    #[test]
    fn missing_version() {
        let vs = vec![1, 2, 3, 4, 5, 6, 8, 9, 10, 11];
        let validator = to_validator(vs);
        let res = validator.missing_versions();
        assert!(
            matches!(res, Err(SourceError::Version(Version { offending_versions, ..})) if offending_versions == vec![8])
        );
    }

    #[test]
    fn source_ok() {
        let vs = vec![1, 2, 3, 4, 5, 6];
        let validator = to_validator(vs);
        let res = validator.validate();
        assert!(res.is_ok())
    }
}
