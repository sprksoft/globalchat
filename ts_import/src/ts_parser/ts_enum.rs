use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use unsynn::{
    unsynn, Assign, BraceGroupContaining, Comma, Cons, Error, LiteralInteger, LiteralString,
    Nothing, Parse, Repeats, ToTokens, TokenIter, Transaction,
};

use crate::Visiblity;

unsynn! {
    pub keyword Enum = "enum";

    pub struct TsEnum {
        enumkw: Enum,
        pub name: Ident,
        variants: BraceGroupContaining<TsEnumVariants>,
    }
    pub enum TsEnumVariants {
        StringBacked(TsEnumItems<StringBacked>),
        NumBacked(TsEnumItems<NumBacked>),
        Normal(TsEnumItems<Nothing>),
    }
    pub struct NumBacked {
        eq: Assign,
        num: LiteralInteger,
    }
    pub struct StringBacked {
        eq: Assign,
        string: LiteralString,
    }
}

#[derive(Debug)]
pub struct TsEnumItems<Backing>(Repeats<1, { usize::MAX }, Cons<Ident, Backing>, Comma>);

impl<B: Backing> TsEnumItems<B> {
    pub fn variant_names(&self) -> impl Iterator<Item = &Ident> {
        self.0 .0.iter().map(|i| &i.value.first)
    }
}

impl<Backing: ToTokens> ToTokens for TsEnumItems<Backing> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}
impl<Backing: ToTokens> quote::ToTokens for TsEnumItems<Backing> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}
impl<Backing: Parse> unsynn::Parser for TsEnumItems<Backing> {
    fn parser(tokens: &mut TokenIter) -> unsynn::Result<Self> {
        Ok(Self(Repeats::<
            1,
            { usize::MAX },
            Cons<Ident, Backing>,
            Comma,
        >::parser(tokens)?))
    }
}

pub trait Backing {
    // Add the type of this backing to the rust enum definition
    // Example:
    // enum Test {
    // Test=0
    // Test1=1
    // Test2=2
    // }
    const ADD_LITERAL_TO_ENUM_VARIANT: bool;
    fn rust_ty() -> TokenStream;
    fn literal(&self) -> Literal;
}
impl Backing for NumBacked {
    const ADD_LITERAL_TO_ENUM_VARIANT: bool = true;
    fn rust_ty() -> TokenStream {
        quote! { isize }
    }
    fn literal(&self) -> Literal {
        self.num.clone().into_inner()
    }
}
impl Backing for StringBacked {
    const ADD_LITERAL_TO_ENUM_VARIANT: bool = false;
    fn rust_ty() -> TokenStream {
        quote! { &'static str }
    }

    fn literal(&self) -> Literal {
        self.string.clone().into_inner()
    }
}

impl TsEnum {
    pub fn variants(&self) -> &TsEnumVariants {
        &self.variants.content
    }

    fn gen_backed_enum<B: Backing>(
        name: &Ident,
        enum_items: &TsEnumItems<B>,
    ) -> (TokenStream, TokenStream) {
        let idents: Vec<&Ident> = enum_items.variant_names().collect();
        let backing: Vec<Literal> = enum_items
            .0
             .0
            .iter()
            .map(|item| item.value.second.literal())
            .collect();

        let enum_content = enum_items.0 .0.iter().map(|item| {
            let name = &item.value.first;
            let backing = &item.value.second.literal();
            if B::ADD_LITERAL_TO_ENUM_VARIANT {
                quote! { #name = #backing, }
            } else {
                quote! { #name, }
            }
        });

        let rust_backing_ty = B::rust_ty();
        let impls = quote! {
            impl #name {
                pub fn to_backing_type(self) -> #rust_backing_ty {
                    match self {
                        #(Self::#idents => #backing,)*
                    }
                }

                pub fn try_from_backing_type(v: #rust_backing_ty) -> Option<Self> {
                    match v {
                        #(#backing => Some(Self::#idents),)*
                        _=>None,
                    }
                }
            }
        };
        (quote! {#(#enum_content)*}, impls)
    }

    pub fn emit(&self, vis: &Visiblity) -> TokenStream {
        let name = &self.name;
        let (enum_content, impls) = match self.variants() {
            TsEnumVariants::Normal(items) => (quote! {#items}, quote! {}),
            TsEnumVariants::NumBacked(items) => Self::gen_backed_enum(name, items),
            TsEnumVariants::StringBacked(items) => Self::gen_backed_enum(name, items),
        };

        quote! {
            #[derive(Debug, Clone, Copy)]
            #vis enum #name {
                #enum_content
            }
            #impls
        }
    }
}

#[cfg(test)]
mod test {
    use unsynn::{Parse, ToTokens};

    use super::{NumBacked, StringBacked, TsEnumItems, TsEnumVariants};

    #[test]
    fn enum_content() {
        let mut tokens = "Test = 5, Test2 = 2".into_token_iter();
        let items = TsEnumVariants::parse(&mut tokens).unwrap();
        match items {
            TsEnumVariants::NumBacked(_) => {
                assert!(true, "NumBacked")
            }
            TsEnumVariants::StringBacked(l) => {
                assert!(false, "StringBacked")
            }
            TsEnumVariants::Normal(_) => {
                assert!(false, "Normal")
            }
        }
    }

    #[test]
    fn backings_num() {
        let mut tokens = "Test = 5, Test2 = 2".into_token_iter();
        let items = TsEnumItems::<NumBacked>::parse(&mut tokens).unwrap();
        assert_eq!(items.0 .0.len(), 2);
    }

    #[test]
    fn backings_string() {
        let mut tokens = "Test = \"Hello\", Test2 = \"World\"".into_token_iter();
        let items = TsEnumItems::<StringBacked>::parse(&mut tokens).unwrap();
        assert_eq!(items.0 .0.len(), 2);
    }
}
