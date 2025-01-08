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
    ) -> Result<Vec<MigrationSource>, ParseError> {
        let location = migration_dir.as_ref().canonicalize().map_err(|e| {
            ParseError::Path(
                format!(
                    "invalid migration path {:?}",
                    migration_dir.as_ref().to_path_buf()
                ),
                e.to_string(),
            )
        })?;
        let mut sources = fs::read_dir(location)
            .map_err(|_| ParseError::Directory("could not read migration directory".to_string()))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .map(Self::parse)
            .collect::<Result<Vec<_>, _>>()?;

        // order asc by version
        sources.sort_by_key(|s| match s {
            Self::Sql(s) => s.version,
            Self::Rs(s) => s.version,
        });

        Ok(sources)
    }

    fn parse(filepath: impl AsRef<Path>) -> Result<Self, ParseError> {
        let filepath = filepath.as_ref();
        let module = filepath.file_stem().ok_or(ParseError::Name(format!(
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
            .ok_or(ParseError::Name(format!(
                r"format is `^V(\d+)__(\w+)\.(sql|rs)$`, got {:?}",
                filepath.to_str(),
            )))?;
        let version: i64 = ver
            .parse()
            .map_err(|_| ParseError::Name("invalid version, expected i64".to_string()))?;
        let source_type = SourceType::from_ext(ext)?;
        let content = fs::read_to_string(filepath).map_err(|e| ParseError::Io(e.to_string()))?;
        let module = module
            .to_str()
            .ok_or(ParseError::Name(
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

#[derive(Debug)]
#[allow(dead_code)]
pub enum ParseError {
    Directory(String),
    Path(String, String),
    Name(String),
    Ext(String),
    Io(String),
    Sql(i64, String),
}

#[derive(Debug, Clone, Copy)]
enum SourceType {
    Sql,
    Rust,
}

impl SourceType {
    pub fn from_ext(ext: &str) -> Result<Self, ParseError> {
        match ext {
            "sql" => Ok(Self::Sql),
            "rs" => Ok(Self::Rust),
            _ => Err(ParseError::Ext(format!(
                "got file extension {ext}, expected `sql` or `rs`"
            ))),
        }
    }
}
