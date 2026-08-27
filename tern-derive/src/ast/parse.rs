use proc_macro2::Span;
use regex::Regex;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;
use syn::Result;
use syn::spanned::Spanned;

const PAT: &str = r#"^(V|UD?|D)(\d+)__(\w+)\.(sql|rs)$"#;

fn filename_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(PAT).unwrap())
}

pub(crate) struct SourceFile {
    pub(crate) ident: syn::Ident,
    pub(crate) path: syn::LitStr,
    pub(crate) content: syn::LitStr,
    pub(crate) modified: syn::LitInt,
    pub(crate) version: i64,
    pub(crate) description: syn::LitStr,
    pub(crate) module: syn::Ident,
    pub(crate) ext: SourceExt,
    pub(crate) typ: SourceType,
}

impl SourceFile {
    pub(crate) fn from_spanned<S: Spanned>(val: &S) -> Result<Self> {
        let span = val.span();
        let pbuf = span.local_file().ok_or_else(|| {
            syn::Error::new(span, "failed to resolve source file")
        })?;
        source_file(pbuf)
    }

    pub(crate) fn from_source_dir<'a>(
        source: &'a syn::LitStr,
    ) -> Result<Box<dyn Iterator<Item = Result<Self>> + 'a>> {
        let iter = dir_entries(source)?.map(|e| e.and_then(from_entry));
        Ok(Box::new(iter))
    }

    pub(crate) fn is_down(&self) -> Option<bool> {
        if self.typ == SourceType::Simple {
            return None;
        }
        Some(!matches!(self.typ, SourceType::Up))
    }
}

fn dir_entries<'a>(
    source: &'a syn::LitStr,
) -> Result<Box<dyn Iterator<Item = Result<DirEntry>> + 'a>> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR unset");
    let parent = PathBuf::from(manifest_dir);
    let path = parent.join(source.value());
    let source_dir = std::fs::read_dir(&path).map_err(|e| {
        syn::Error::new(
            source.span(),
            format!(
                "error reading source dir from {}: {e}",
                path.as_os_str().display()
            ),
        )
    })?;
    let iter = source_dir.filter_map(|entry| {
        match entry.map_err(|e| {
            syn::Error::new(
                source.span(),
                format!("error with source entry: {e}"),
            )
        }) {
            Err(e) => Some(Err(e)),
            Ok(val)
                if val.file_name().to_str().is_some_and(|f| {
                    !(f == "mod.rs" || f.starts_with("."))
                }) =>
            {
                Some(Ok(val))
            },
            _ => None,
        }
    });
    Ok(Box::new(iter))
}

fn from_entry(entry: DirEntry) -> Result<SourceFile> {
    source_file(entry.path())
}

fn source_file(pbuf: PathBuf) -> Result<SourceFile> {
    let modified = modified(&pbuf);
    let file_name =
        pbuf.as_path().file_name().and_then(OsStr::to_str).ok_or_else(
            || syn::Error::new(Span::call_site(), "invalid path"),
        )?;

    let re = filename_re();

    let capt = re.captures(file_name).ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            format!("invalid name: expected {PAT}"),
        )
    })?;

    let version = capt
        .get(2)
        .and_then(|m| m.as_str().parse::<i64>().ok())
        .ok_or_else(|| syn::Error::new(Span::call_site(), "invalid version"))?;

    let description = capt
        .get(3)
        .map(|d| syn::LitStr::new(d.as_str(), Span::call_site()))
        .ok_or_else(|| {
            syn::Error::new(Span::call_site(), "invalid description")
        })?;

    let ext = capt.get(4).and_then(|m| SourceExt::new(m.as_str())).ok_or_else(
        || syn::Error::new(Span::call_site(), "invalid extension"),
    )?;

    let typ =
        capt.get(1).and_then(|m| SourceType::new(m.as_str())).ok_or_else(
            || syn::Error::new(Span::call_site(), "invalid source type"),
        )?;

    let module = pbuf
        .as_path()
        .file_stem()
        .and_then(OsStr::to_str)
        .map(|s| syn::Ident::new(s, Span::call_site()))
        .ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                format!("{typ}{version} filename invalid"),
            )
        })?;

    let pstr = pbuf.to_string_lossy().to_string();
    let path = syn::LitStr::new(&pstr, module.span());

    let ident_str = format!("___Resolve{}{}", typ, version);
    let ident = syn::Ident::new(&ident_str, Span::call_site());

    let content = if ext.is_sql() {
        std::fs::read_to_string(&pbuf)
            .map_err(|e| syn::Error::new(Span::call_site(), e.to_string()))
            .map(|s| syn::LitStr::new(&s, module.span()))?
    } else {
        syn::LitStr::new("", module.span())
    };

    Ok(SourceFile {
        ident,
        path,
        content,
        modified,
        version,
        description,
        module,
        ext,
        typ,
    })
}

// We write this in a `#[doc = #modified]` in the emitted code coming from a
// .sql migration.  There are some unknown edge cases where .sql changes don't
// trigger a rebuild even with a build.rs watching the migrations.  This seems
// to fix it.
fn modified(path: &Path) -> syn::LitInt {
    let sec = path
        .metadata()
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .and_then(|d| Some(d.as_secs()))
        .unwrap_or_default()
        .to_string();
    syn::LitInt::new(&sec, Span::call_site())
}

/// Type of the file.  `Sql` contains static SQL always, `Rs` may be either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceExt {
    Rs,
    Sql,
}

impl SourceExt {
    fn new(ext: &str) -> Option<Self> {
        Some(match ext {
            "rs" => Self::Rs,
            "sql" => Self::Sql,
            _ => return None,
        })
    }

    fn is_sql(&self) -> bool {
        *self == SourceExt::Sql
    }
}

impl Display for SourceExt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Rs => "rs",
            Self::Sql => "sql",
        };
        f.write_str(s)
    }
}

/// Type of migration set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SourceType {
    Up,
    Down,
    UpDown,
    #[default]
    Simple,
}

impl SourceType {
    fn new(capt: &str) -> Option<Self> {
        Some(match capt {
            "U" => Self::Up,
            "D" => Self::Down,
            "UD" => Self::UpDown,
            "V" => Self::Simple,
            _ => return None,
        })
    }
}

impl Display for SourceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Up => "U",
            Self::Down => "D",
            Self::UpDown => "UD",
            Self::Simple => "V",
        };
        f.write_str(s)
    }
}
