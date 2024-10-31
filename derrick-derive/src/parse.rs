use regex::Regex;
use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

fn filename_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^V(\d+)__(\w+)\.(sql|rs)$").unwrap())
}

pub fn cargo_manifest_dir() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    PathBuf::from(manifest_dir)
}

#[derive(Debug)]
pub struct SourceToken {
    pub runtime: syn::Ident,
    pub loc: syn::LitStr,
}

impl SourceToken {
    pub fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let runtime = &input.ident;
        let mut loc_arg = None::<syn::LitStr>;
        for attr in &input.attrs {
            if attr.path().is_ident("migration") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("path") {
                        let loc_attr_val: syn::LitStr = meta.value()?.parse()?;
                        loc_arg = Some(loc_attr_val);
                    }

                    Ok(())
                })?
            }
        }
        let loc = loc_arg.ok_or(syn::Error::new(
            input.ident.span(),
            "arg `path = migrations/dir/from/crate/root/` is required",
        ))?;

        Ok(Self {
            runtime: runtime.clone(),
            loc,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ParsedSource {
    pub module: String,
    pub source_type: SourceType,
    pub version: i64,
    pub description: String,
    pub content: String,
}

impl ParsedSource {
    pub fn from_migration_dir(loc: impl AsRef<Path>) -> Result<Vec<ParsedSource>, ParseError> {
        let location = loc.as_ref().canonicalize().map_err(|e| {
            ParseError::Path(
                format!("invalid migration path {:?}", loc.as_ref().to_path_buf()),
                e.to_string(),
            )
        })?;
        let mut sources = fs::read_dir(location)
            .map_err(|_| ParseError::Directory("could not read migration directory".to_string()))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .map(Self::parse)
            .collect::<Result<Vec<_>, _>>()?;

        // order asc by version
        sources.sort_by_key(|s| s.version);

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
        let content = fs::read_to_string(filepath)
            .map_err(|e| ParseError::Content(format!("could not read file {:?}", e)))?;
        let module = module
            .to_str()
            .ok_or(ParseError::Content(
                "utf-8 decoding filename failed".to_string(),
            ))?
            .to_string();

        Ok(Self {
            module,
            source_type,
            version,
            content,
            description: description.to_string(),
        })
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum ParseError {
    Directory(String),
    Path(String, String),
    Name(String),
    Ext(String),
    Content(String),
}

#[derive(Debug, Clone, Copy)]
pub enum SourceType {
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
