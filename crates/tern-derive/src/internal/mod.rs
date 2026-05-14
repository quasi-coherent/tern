use proc_macro2::Span;
use std::fmt::{self, Display, Formatter};
use syn::Result;
use syn::spanned::Spanned;

pub(crate) mod ast;

mod source_file;
pub(crate) use source_file::SourceFile;

mod source_mods;
pub(crate) use source_mods::SourceMods;

/// A map_err/ok_or_else to syn::Error convenience.
pub(crate) trait IntoResult<T>: Sized {
    /// Convert to a `syn::Result`.
    fn into_result<D: Into<String>>(self, span: Span, msg: D) -> Result<T>;

    fn result_msg<D: Into<String>>(self, msg: D) -> Result<T> {
        self.into_result(Span::call_site(), msg)
    }

    fn result_msgv<D: Into<String>>(self, ver: i64, msg: D) -> Result<T>
    where
        Self: Sized,
    {
        let msg = format!("invalid version: {}: {ver}", msg.into());
        self.result_msg(msg)
    }

    fn result_sp<S: Spanned, D: Into<String>>(
        self,
        sp: &S,
        msg: D,
    ) -> Result<T> {
        self.into_result(sp.span(), msg)
    }
}

impl<T> IntoResult<T> for Option<T> {
    fn into_result<D: Into<String>>(self, span: Span, msg: D) -> Result<T> {
        self.ok_or_else(|| syn::Error::new(span, msg.into()))
    }
}

impl<T, E: std::fmt::Display> IntoResult<T> for std::result::Result<T, E> {
    fn into_result<D: Into<String>>(self, span: Span, msg: D) -> Result<T> {
        self.map_err(|e| syn::Error::new(span, format!("{}: {e}", msg.into())))
    }
}

/// Type of the file.  `Sql` contains static SQL always, `Rs` may be either.
#[derive(Debug, Clone, Copy)]
enum SourceExt {
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
enum SourceType {
    Up,
    Down,
    #[default]
    Simple,
}

impl SourceType {
    fn new(capt: &str) -> Option<Self> {
        Some(match capt {
            "U" => Self::Up,
            "D" => Self::Down,
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
            Self::Simple => "V",
        };
        f.write_str(s)
    }
}
