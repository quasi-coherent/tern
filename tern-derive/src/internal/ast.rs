use syn::Result;
use syn::spanned::Spanned;

/// Parse attributes found at any level of the ast.
///
/// It's always the same procedure, just a different starting input (represented by `I`).
pub trait ParseAttr<I>: Default {
    /// Get correct level of attributes.
    fn attrs(input: &I) -> impl Iterator<Item = &syn::Attribute>;

    /// Logic to update based on matching `attr`s.
    fn update(&mut self, attr: &syn::Attribute) -> Result<()>;

    fn parse_ast(input: &I) -> Result<Self>
    where
        Self: Sized,
    {
        let mut init = Self::default();
        for attr in Self::attrs(input) {
            init.update(attr)?;
        }
        Ok(init)
    }
}

/// Skip attribute parsing.
#[derive(Default)]
pub struct SkipParseAttr;

impl ParseAttr<syn::DeriveInput> for SkipParseAttr {
    fn attrs(_: &syn::DeriveInput) -> impl Iterator<Item = &syn::Attribute> {
        std::iter::empty::<&syn::Attribute>()
    }
    fn update(&mut self, _: &syn::Attribute) -> Result<()> {
        Ok(())
    }
}

impl ParseAttr<syn::Field> for SkipParseAttr {
    fn attrs(_: &syn::Field) -> impl Iterator<Item = &syn::Attribute> {
        std::iter::empty::<&syn::Attribute>()
    }
    fn update(&mut self, _: &syn::Attribute) -> Result<()> {
        Ok(())
    }
}

/// Token stream parsed from the `syn::DeriveInput`.
pub struct Container<'a, Der, Fld> {
    /// The type having the derive.
    pub ty: Type<'a>,
    /// Derive attributes.
    pub attrs: Der,
    /// The fields of the struct.
    pub fields: Vec<Field<'a, Fld>>,
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
            _ => {
                Err(syn::Error::new(input.span(), "only structs are supported"))
            },
        }?;

        Ok(Self { ty, attrs, fields })
    }

    /// Fields of a struct parsed from the `syn::DeriveInput`.
    fn fields(input: &'a syn::Fields) -> Result<Vec<Field<'a, Fld>>> {
        let fields = match &input {
            syn::Fields::Named(fields_named) => fields_named.named.iter(),
            syn::Fields::Unnamed(fields_unnamed) => {
                fields_unnamed.unnamed.iter()
            },
            syn::Fields::Unit => {
                return Ok(Vec::new());
            },
        }
        .enumerate()
        .map(|(i, input)| Field::parse_ast(i, input))
        .collect::<Result<Vec<_>>>()?;

        Ok(fields)
    }
}

/// The deriving type.
pub struct Type<'a> {
    /// The name of the type.
    pub ident: &'a syn::Ident,
    // TODO
    #[allow(dead_code)]
    generics: &'a syn::Generics,
}

/// A field in the struct.
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
}
