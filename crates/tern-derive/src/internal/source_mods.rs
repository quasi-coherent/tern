use proc_macro2::{Span, TokenStream};
use std::collections::BTreeMap;
use std::fs::DirEntry;
use std::path::PathBuf;
use syn::Result;

use super::{IntoResult as _, SourceFile, SourceType};

/// Token stream of a module with impl `Migration` per source file.
pub enum SourceMods {
    Simple(SourceModTokens),
    UpDown { up: SourceModTokens, down: SourceModTokens },
}

impl SourceMods {
    /// From the `source` attribute value of `TernMigrate` derive.
    pub fn new(ident: &syn::Ident, source: &syn::LitStr) -> Result<Self> {
        let map = SourceMap::new(source)?;
        if map.simple.is_empty() {
            let iter = map.up.values();
            let (up, down) =
                SourceModTokens::new_up_down(ident, iter, &map.down)?;
            return Ok(Self::UpDown { up, down });
        }
        let iter = map.simple.values();
        let tokens = SourceModTokens::new_simple(ident, iter)?;
        Ok(Self::Simple(tokens))
    }

    /// Return the token stream containing either `TernMigrate` or `TernMigrate`
    /// and `Invertible` implementations, plus module declarations.
    pub fn quot_impl_tern_mods(&self, ident: &syn::Ident) -> TokenStream {
        match self {
            Self::Simple(tokens) => {
                let mods = &tokens.mods;
                let migs = &tokens.migrations;
                quote::quote! {
                    #mods
                    impl ::tern::TernApp for #ident {
                        fn up_migrations(&self) -> ::tern::migration::iter::UpMigrationSet<Self> {
                            ::tern::migration::iter::UpMigrationSet::new(vec![#(#migs),*])
                        }
                    }
                }
            },
            Self::UpDown { up, down } => {
                let umods = &up.mods;
                let umigs = &up.migrations;
                let dmods = &down.mods;
                let dmigs = &down.migrations;
                quote::quote! {
                    #umods
                    #dmods
                    impl ::tern::TernApp for #ident {
                        fn up_migrations(&self) -> ::tern::migration::iter::UpMigrationSet<Self> {
                            ::tern::migration::iter::UpMigrationSet::new(vec![#(#umigs),*])
                        }
                        fn down_migrations(&self) -> ::std::option::Option<::tern::migration::iter::DownMigrationSet<Self>> {
                            ::std::option::Option::Some(::tern::migration::iter::DownMigrationSet::new(vec![#(#dmigs),*]))
                        }
                    }
                }
            },
        }
    }
}

#[derive(Default)]
pub struct SourceModTokens {
    migrations: Vec<TokenStream>,
    mods: TokenStream,
}

impl SourceModTokens {
    fn new_simple<'a, T>(ident: &syn::Ident, mut iter: T) -> Result<Self>
    where
        T: Iterator<Item = &'a SourceFile>,
    {
        iter.try_fold(Self::default(), |mut acc, f| {
            acc.push(ident, f)?;
            Ok(acc)
        })
    }

    fn new_up_down<'a, T>(
        ident: &syn::Ident,
        mut iter: T,
        down: &BTreeMap<i64, SourceFile>,
    ) -> Result<(Self, Self)>
    where
        T: Iterator<Item = &'a SourceFile>,
    {
        let mut up_tok = SourceModTokens::default();
        let mut down_tok = SourceModTokens::default();
        iter.try_for_each(|u| {
            let d = down
                .get(&u.version)
                .result_msgv(u.version, "missing down migration")?;
            up_tok.push(ident, u)?;
            down_tok.push(ident, d)?;
            Ok::<_, syn::Error>(())
        })?;
        Ok((up_tok, down_tok))
    }

    fn push(&mut self, ident: &syn::Ident, file: &SourceFile) -> Result<()> {
        let mg = file.quot_migration_expr();
        let md = file.quot_mod(ident)?;
        let mods = &self.mods;
        let nmods = quote::quote! {
            #mods
            #md
        };
        self.migrations.push(mg);
        self.mods = nmods;
        Ok(())
    }
}

// Mapping of version to source file data.
#[derive(Default)]

struct SourceMap {
    simple: BTreeMap<i64, SourceFile>,
    up: BTreeMap<i64, SourceFile>,
    down: BTreeMap<i64, SourceFile>,
}

impl SourceMap {
    fn new(source: &syn::LitStr) -> Result<Self> {
        let source_dir = Self::iter_source_dir(source)?;
        let this = Self::from_dir_entries(source_dir)?;
        this.validate()?;
        Ok(this)
    }

    fn from_dir_entries<T>(mut iter: T) -> Result<Self>
    where
        T: Iterator<Item = Result<DirEntry>>,
    {
        iter.try_fold(Self::default(), |mut acc, res| {
            let file = res.and_then(SourceFile::from_entry)?;
            acc.insert(file)?;
            Ok(acc)
        })
    }

    fn insert(&mut self, file: SourceFile) -> Result<()> {
        let ret = match file.typ {
            SourceType::Simple
                if self.up.is_empty() && self.down.is_empty() =>
            {
                self.simple.insert(file.version, file)
            },
            SourceType::Up if self.simple.is_empty() => {
                self.up.insert(file.version, file)
            },
            SourceType::Down if self.simple.is_empty() => {
                self.down.insert(file.version, file)
            },
            _ => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "found mixed migration types",
                ));
            },
        };
        if let Some(f) = ret {
            Err(syn::Error::new(
                Span::call_site(),
                format!("duplicate version {}", f.version),
            ))
        } else {
            Ok(())
        }
    }

    fn validate(&self) -> Result<()> {
        if !self.simple.is_empty() {
            return Self::check_contiguous(self.simple.keys(), |_| Ok(()));
        }

        Self::check_contiguous(self.up.keys(), |k| {
            if !self.down.contains_key(&k) {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!("U/D migration missing for version {k}"),
                ));
            }

            Ok(())
        })
    }

    fn check_contiguous<'a, T, F>(mut iter: T, mut f: F) -> Result<()>
    where
        T: Iterator<Item = &'a i64>,
        F: FnMut(i64) -> Result<()>,
    {
        iter.try_fold(None::<i64>, |prev, curr| {
            let c = *curr;
            if let Some(l) = prev
                && c != l + 1
            {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!("missing version {}", l + 1),
                ));
            }
            f(c)?;
            Ok(Some(c))
        })?;

        Ok(())
    }

    fn iter_source_dir<'a>(
        source: &'a syn::LitStr,
    ) -> Result<Box<dyn Iterator<Item = Result<DirEntry>> + 'a>> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR not set");
        let parent = PathBuf::from(manifest_dir);
        let path = parent.join(source.value());
        let source_dir = std::fs::read_dir(&path).result_sp(
            source,
            format!(
                "error reading source dir from {}",
                path.as_os_str().display()
            ),
        )?;
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
}
