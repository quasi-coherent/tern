use syn::Result;
use syn::spanned::Spanned;

/// Token stream parsed from the `syn::DeriveInput`.
pub struct Container<'a, Der, Fld> {
    /// The type having the derive.
    pub ty: Type<'a>,
    /// Derive attributes.
    pub attrs: Der,
    /// The fields of the struct.
    pub fields: Fields<'a, Fld>,
}

impl<'a, Der, Fld> Container<'a, Der, Fld>
where
    Der: ParseAttr<syn::DeriveInput>,
    Fld: ParseAttr<syn::Field>,
{
    pub fn from_ast(input: &'a syn::DeriveInput) -> Result<Self> {
        let ident = &input.ident;
        let generics = &input.generics;
        let ty = Type { ident, generics };
        let attrs = Der::parse_ast(input)?;
        let fields = match &input.data {
            syn::Data::Struct(ds) => Self::fields(&ds.fields),
            _ => Err(syn::Error::new(input.span(), "only structs are supported")),
        }?;

        Ok(Self { ty, attrs, fields })
    }

    /// Fields of a struct parsed from the `syn::DeriveInput`.
    fn fields(input: &'a syn::Fields) -> Result<Fields<'a, Fld>> {
        let style: Style;
        let fields = match &input {
            syn::Fields::Named(fields_named) => {
                style = Style::Named;
                fields_named.named.iter()
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                style = if fields_unnamed.unnamed.len() == 1 {
                    Style::Newtype
                } else {
                    Style::Tuple
                };
                fields_unnamed.unnamed.iter()
            }
            syn::Fields::Unit => {
                return Ok(Fields {
                    style: Style::Unit,
                    fields: Vec::new(),
                });
            }
        }
        .enumerate()
        .map(|(i, input)| Field::parse_ast(i, input))
        .collect::<Result<Vec<_>>>()?;

        Ok(Fields { style, fields })
    }
}

/// The deriving type.
pub struct Type<'a> {
    /// The name of the type.
    pub ident: &'a syn::Ident,
    /// If it has any generic parameters and/or lifetimes.
    #[allow(dead_code)]
    pub generics: &'a syn::Generics,
}

/// The style of struct.
#[derive(Debug, Clone, Copy)]
pub enum Style {
    /// Named fields.
    Named,
    /// One unnamed field.
    Newtype,
    /// More than one unnamed field.
    Tuple,
    /// No fields.
    Unit,
}

/// All the fields of the struct.
#[derive(Clone)]
pub struct Fields<'a, Fld> {
    #[allow(dead_code)]
    pub style: Style,
    pub fields: Vec<Field<'a, Fld>>,
}

/// A field in the struct.
#[derive(Clone)]
pub struct Field<'a, Fld> {
    pub member: syn::Member,
    pub ty: &'a syn::Type,
    pub attrs: Fld,
}

impl<'a, Fld> Field<'a, Fld>
where
    Fld: ParseAttr<syn::Field>,
{
    fn parse_ast(ix: usize, input: &'a syn::Field) -> Result<Self> {
        let member = match &input.ident {
            Some(ident) => syn::Member::Named(ident.clone()),
            _ => syn::Member::Unnamed(ix.into()),
        };
        let ty = &input.ty;
        let attrs = Fld::parse_ast(input)?;
        Ok(Self { member, ty, attrs })
    }

    #[allow(dead_code)]
    pub fn is_option(&self) -> bool {
        matches!(self.ty, syn::Type::Path(p) if p.path.segments.first().is_some_and(|s| s.ident == "Option"))
    }
}

/// Parse attributes found at any level of the ast.
///
/// It's always the same procedure, just a different starting input (represented by `I`).
pub trait ParseAttr<I> {
    /// Initialize empty attributes.
    fn init() -> Self;

    /// Get correct level of attributes.
    fn attrs(input: &I) -> impl Iterator<Item = &syn::Attribute>;

    /// Logic to update based on matching `attr`s.
    fn update(&mut self, attr: &syn::Attribute) -> Result<()>
    where
        Self: Sized;

    fn parse_ast(input: &I) -> Result<Self>
    where
        Self: Sized,
    {
        let mut init = Self::init();
        for attr in Self::attrs(input) {
            init.update(attr)?;
        }
        Ok(init)
    }
}

/// Skip attribute parsing.
pub struct SkipParseAttr;

impl ParseAttr<syn::DeriveInput> for SkipParseAttr {
    fn init() -> Self {
        SkipParseAttr
    }
    fn attrs(_: &syn::DeriveInput) -> impl Iterator<Item = &syn::Attribute> {
        std::iter::empty::<&syn::Attribute>()
    }
    fn update(&mut self, _: &syn::Attribute) -> Result<()> {
        Ok(())
    }
}

impl ParseAttr<syn::Field> for SkipParseAttr {
    fn init() -> Self {
        SkipParseAttr
    }
    fn attrs(_: &syn::Field) -> impl Iterator<Item = &syn::Attribute> {
        std::iter::empty::<&syn::Attribute>()
    }
    fn update(&mut self, _: &syn::Attribute) -> Result<()> {
        Ok(())
    }
}
