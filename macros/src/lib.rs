use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Error, Parse, ParseStream, Result};

#[derive(Debug, PartialEq)]
struct HeaderItem {
    order: usize,
    name: String,
    content: String,
}

impl Parse for HeaderItem {
    fn parse(input: ParseStream) -> Result<Self> {
        // first, try matching an item
        match input.parse::<syn::Item>() {
            Ok(item) => {
                return match item {
                    syn::Item::Fn(item) => {
                        let name = item.sig.ident.to_string();
                        HeaderItem::from_parts(name, &item.attrs)
                    }
                    syn::Item::Const(item) => {
                        let name = item.ident.to_string();
                        HeaderItem::from_parts(name, &item.attrs)
                    }
                    syn::Item::Static(item) => {
                        let name = item.ident.to_string();
                        HeaderItem::from_parts(name, &item.attrs)
                    }
                    syn::Item::Struct(item) => {
                        let name = item.ident.to_string();
                        HeaderItem::from_parts(name, &item.attrs)
                    }
                    syn::Item::Enum(item) => {
                        let name = item.ident.to_string();
                        HeaderItem::from_parts(name, &item.attrs)
                    }
                    syn::Item::Union(item) => {
                        let name = item.ident.to_string();
                        HeaderItem::from_parts(name, &item.attrs)
                    }
                    syn::Item::Type(item) => {
                        let name = item.ident.to_string();
                        HeaderItem::from_parts(name, &item.attrs)
                    }
                    _ => HeaderItem::errmsg("cannot determine header content from this item"),
                }
            }
            Err(_) => {}
        }
        println!("{:?}", input);
        todo!()
    }
}

impl HeaderItem {
    fn errmsg<T: std::fmt::Display>(msg: T) -> Result<Self> {
        Result::Err(Error::new(Span::call_site(), msg))
    }

    /// Create a HeaderItem from a Rust item, given its name and a vec of its attributes.
    fn from_parts(name: String, attrs: &Vec<syn::Attribute>) -> Result<Self> {
        // extract any docstring from the attributes
        let mut doc = vec![];
        for attr in attrs {
            if let Ok(syn::Meta::NameValue(nv)) = attr.parse_meta() {
                if nv.path.is_ident("doc") {
                    if let syn::Lit::Str(s) = nv.lit {
                        doc.push(s.value());
                    }
                }
            }
        }

        if doc.len() == 0 {
            return HeaderItem::errmsg(format!("{} does not have a docstring", name));
        }

        Ok(HeaderItem::parse_docstring(name, doc))
    }

    /// Parse a docstring, presented as a vec of lines, to extract C declarations and comments.
    fn parse_docstring(name: String, doc: Vec<String>) -> HeaderItem {
        // TODO: strip common leading whitespace from all lines, leading/trailing empty
        // comment lines
        let mut content = vec![];
        let mut decl = false;
        for line in doc {
            if decl {
                if line.trim() == "```" {
                    decl = false;
                    continue;
                }
                content.push(line);
            } else {
                if line.trim() == "```c" {
                    decl = true;
                    continue;
                }
                content.push(format!("//{}", line));
            }
        }

        HeaderItem {
            order: 100, // default
            name,
            content: itertools::join(content, "\n"),
        }
    }

    /// Convert this HeaderItem into a TokenStream that will include it in the built binary.
    fn into_tokens(self) -> TokenStream2 {
        let HeaderItem {
            order,
            name,
            content,
        } = self;
        let item_name = syn::Ident::new(&format!("FFIZZ_HDR__{}", name), Span::call_site());

        quote! {
            #[::ffizz_header::linkme::distributed_slice(::ffizz_header::FFIZZ_HEADER_ITEMS)]
            #[linkme(crate=::ffizz_header::linkme)]
            static #item_name: ::ffizz_header::HeaderItem = ::ffizz_header::HeaderItem {
                order: #order,
                name: #name,
                content: #content,
            };
        }
        .into()
    }
}

/// TODO: doc (does that show up in the re-import as ffizz_header::item?)
#[proc_macro_attribute]
pub fn item(attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let parsed = {
        let item = item.clone();
        syn::parse_macro_input!(item as HeaderItem)
    };
    item.extend(TokenStream::from(parsed.into_tokens()));
    item
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parsing_fn() {
        let hi: HeaderItem = syn::parse_quote! {
            /// A docstring
            pub unsafe extern "C" fn add(x: u32, y: u32) -> u32 {}
        };
        assert_eq!(
            hi,
            HeaderItem {
                order: 100,
                name: "add".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_const() {
        let hi: HeaderItem = syn::parse_quote! {
            /// A docstring
            pub const X: usize = 13;
        };
        assert_eq!(
            hi,
            HeaderItem {
                order: 100,
                name: "X".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_static() {
        let hi: HeaderItem = syn::parse_quote! {
            /// A docstring
            pub static X: usize = 13;
        };
        assert_eq!(
            hi,
            HeaderItem {
                order: 100,
                name: "X".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_struct() {
        let hi: HeaderItem = syn::parse_quote! {
            /// A docstring
            pub struct Foo {}
        };
        assert_eq!(
            hi,
            HeaderItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_enum() {
        let hi: HeaderItem = syn::parse_quote! {
            /// A docstring
            pub enum Foo {}
        };
        assert_eq!(
            hi,
            HeaderItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_union() {
        let hi: HeaderItem = syn::parse_quote! {
            /// A docstring
            pub union Foo {}
        };
        assert_eq!(
            hi,
            HeaderItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_type() {
        let hi: HeaderItem = syn::parse_quote! {
            /// A docstring
            pub type Foo = Bar;
        };
        assert_eq!(
            hi,
            HeaderItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    //#[test]
    fn test_parsing_inner() {
        let hi: HeaderItem = syn::parse_quote! {
            //! A docstring
        };
        assert_eq!(
            hi,
            HeaderItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }
}
