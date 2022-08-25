use crate::headeritem::HeaderItem;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::parse::{Error, Parse, ParseStream, Result};

/// DocItem is the result of parsing an item, with a header_item constructed from the
/// item's docstrings and any ffizz-related attributes.
#[derive(Debug, PartialEq)]
pub(crate) struct DocItem {
    header_item: HeaderItem,
    syn_item: syn::Item,
}

impl Parse for DocItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut item = input.parse::<syn::Item>()?;
        let (name, attrs) = match &mut item {
            syn::Item::Fn(item) => (item.sig.ident.to_string(), &mut item.attrs),
            syn::Item::Const(item) => (item.ident.to_string(), &mut item.attrs),
            syn::Item::Static(item) => (item.ident.to_string(), &mut item.attrs),
            syn::Item::Struct(item) => (item.ident.to_string(), &mut item.attrs),
            syn::Item::Enum(item) => (item.ident.to_string(), &mut item.attrs),
            syn::Item::Union(item) => (item.ident.to_string(), &mut item.attrs),
            syn::Item::Type(item) => (item.ident.to_string(), &mut item.attrs),
            // TODO: Use (no support for globs or groups)
            _ => {
                return Err(Error::new_spanned(
                    item,
                    "cannot determine header content from this item",
                ));
            }
        };

        Ok(DocItem {
            header_item: HeaderItem::from_attrs(name, attrs)?,
            syn_item: item,
        })
    }
}

impl DocItem {
    /// Convert this DocItem into a TokenStream that will include it in the built binary.
    pub(crate) fn to_tokens(self, tokens: &mut TokenStream2) {
        self.syn_item.to_tokens(tokens);
        self.header_item.to_tokens(tokens);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parsing_fn() {
        let di: DocItem = syn::parse_quote! {
            /// A docstring
            pub unsafe extern "C" fn add(x: u32, y: u32) -> u32 {}
        };
        assert_eq!(
            di.header_item,
            HeaderItem {
                order: 100,
                name: "add".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_const() {
        let di: DocItem = syn::parse_quote! {
            /// A docstring
            pub const X: usize = 13;
        };
        assert_eq!(
            di.header_item,
            HeaderItem {
                order: 100,
                name: "X".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_static() {
        let di: DocItem = syn::parse_quote! {
            /// A docstring
            pub static X: usize = 13;
        };
        assert_eq!(
            di.header_item,
            HeaderItem {
                order: 100,
                name: "X".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_struct() {
        let di: DocItem = syn::parse_quote! {
            /// A docstring
            pub struct Foo {}
        };
        assert_eq!(
            di.header_item,
            HeaderItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_enum() {
        let di: DocItem = syn::parse_quote! {
            /// A docstring
            pub enum Foo {}
        };
        assert_eq!(
            di.header_item,
            HeaderItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_union() {
        let di: DocItem = syn::parse_quote! {
            /// A docstring
            pub union Foo {}
        };
        assert_eq!(
            di.header_item,
            HeaderItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_type() {
        let di: DocItem = syn::parse_quote! {
            /// A docstring
            pub type Foo = Bar;
        };
        assert_eq!(
            di.header_item,
            HeaderItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_type_with_attrs() {
        let di: DocItem = syn::parse_quote! {
            /// A docstring
            #[ffizz(name="bar", order=10)]
            fn foo() {}
        };
        assert_eq!(
            di.header_item,
            HeaderItem {
                order: 10,
                name: "bar".into(),
                content: "// A docstring".into(),
            }
        );
    }
}
