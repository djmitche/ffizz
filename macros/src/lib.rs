use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::parse::{Error, Parse, ParseStream, Result};

/// HeaderItem is an in-memory version of the HeaderItem object these macros
/// will insert into the Rust code
#[derive(Debug, PartialEq)]
struct HeaderItem {
    order: usize,
    name: String,
    content: String,
}

impl HeaderItem {
    /// Create a HeaderItem from a Rust item, given its name and a vec of its attributes.
    /// All ffizz_header-specific attributes are removed from attrs.
    fn from_attrs(mut name: String, attrs: &mut Vec<syn::Attribute>) -> Result<Self> {
        let mut order = 100; // default

        // extract docstring segments and header_name / header_order attributes
        // TODO: /*! ... */ can be multi-line, so split those
        let mut doc = vec![];
        let mut kept_attrs = vec![];
        for attr in attrs.drain(..) {
            if let Ok(syn::Meta::NameValue(nv)) = attr.parse_meta() {
                let mut keep_attr = true;
                if nv.path.is_ident("doc") {
                    if let syn::Lit::Str(s) = nv.lit {
                        doc.push(s.value());
                    }
                } else if nv.path.is_ident("header_name") {
                    keep_attr = false;
                    if let syn::Lit::Str(ref s) = nv.lit {
                        name = s.value();
                    } else {
                        return Self::errmsg("usage: #[header_name = \"...\"]");
                    }
                } else if nv.path.is_ident("header_order") {
                    keep_attr = false;
                    if let syn::Lit::Int(i) = nv.lit {
                        if let Ok(i) = i.base10_parse::<usize>() {
                            order = i;
                        } else {
                            return Self::errmsg("usage: #[header_order = 1234]");
                        }
                    } else {
                        return Self::errmsg("usage: #[header_order = 1234]");
                    }
                }
                if keep_attr {
                    kept_attrs.push(attr);
                }
            }
        }
        *attrs = kept_attrs;

        if doc.len() == 0 {
            return Self::errmsg(format!("{} does not have a docstring", name));
        }

        Ok(HeaderItem::parse_docstring(name, order, doc))
    }

    /// Parse a docstring, presented as a vec of lines, to extract C declarations and comments.
    fn parse_docstring(name: String, order: usize, doc: Vec<String>) -> Self {
        // TODO: strip common leading whitespace from all lines, leading/trailing empty
        // comment lines
        // TODO: parse _AND REMOVE_ #[header_order(10)] for order
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
            order,
            name,
            content: itertools::join(content, "\n"),
        }
    }

    /// Convert this HeaderItem into a TokenStream that will include it in the built binary.
    fn to_tokens(self, tokens: &mut TokenStream2) {
        let HeaderItem {
            order,
            name,
            content,
        } = self;
        let item_name = syn::Ident::new(&format!("FFIZZ_HDR__{}", name), Span::call_site());

        // insert an invocation of linkme::distributed_slice to add this header item to
        // the FFIZZ_HEADER_ITEMS slice.
        tokens.extend(quote! {
            #[::ffizz_header::linkme::distributed_slice(::ffizz_header::FFIZZ_HEADER_ITEMS)]
            #[linkme(crate=::ffizz_header::linkme)]
            static #item_name: ::ffizz_header::HeaderItem = ::ffizz_header::HeaderItem {
                order: #order,
                name: #name,
                content: #content,
            };
        });
    }

    fn errmsg<T: std::fmt::Display>(msg: T) -> Result<Self> {
        Result::Err(Error::new(Span::call_site(), msg))
    }
}

/// DocItem is a syn Item with documentation attached.
#[derive(Debug, PartialEq)]
struct DocItem {
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
                println!("{:?}", item);
                return Self::errmsg("cannot determine header content from this item");
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
    fn to_tokens(self, tokens: &mut TokenStream2) {
        self.syn_item.to_tokens(tokens);
        self.header_item.to_tokens(tokens);
    }

    fn errmsg<T: std::fmt::Display>(msg: T) -> Result<Self> {
        Result::Err(Error::new(Span::call_site(), msg))
    }
}

/// TODO: doc (does that show up in the re-import as ffizz_header::item?)
#[proc_macro_attribute]
pub fn item(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let docitem = syn::parse_macro_input!(item as DocItem);
    let mut tokens = TokenStream2::new();
    docitem.to_tokens(&mut tokens);
    tokens.into()
}

/// Snippet is just a header snippet, with no associated Rust syntax.
#[derive(Debug, PartialEq)]
struct Snippet {
    header_item: HeaderItem,
}

impl Parse for Snippet {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = input.call(syn::Attribute::parse_outer)?;
        let header_item = HeaderItem::from_attrs(String::new(), &mut attrs)?;
        if header_item.name.len() == 0 {
            return Self::errmsg("snippet! requires a name (`#[snippet(name=\"..\")]`)");
        }
        Ok(Snippet { header_item })
    }
}

impl Snippet {
    /// Convert this DocItem into a TokenStream that will include it in the built binary.
    fn to_tokens(self, tokens: &mut TokenStream2) {
        self.header_item.to_tokens(tokens);
    }

    fn errmsg<T: std::fmt::Display>(msg: T) -> Result<Self> {
        Result::Err(Error::new(Span::call_site(), msg))
    }
}

/// TODO: doc
#[proc_macro]
pub fn snippet(item: TokenStream) -> TokenStream {
    let snippet = syn::parse_macro_input!(item as Snippet);
    let mut tokens = TokenStream2::new();
    snippet.to_tokens(&mut tokens);
    tokens.into()
}

/*
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parsing_fn() {
        let hi: DocItem = syn::parse_quote! {
            /// A docstring
            pub unsafe extern "C" fn add(x: u32, y: u32) -> u32 {}
        };
        assert_eq!(
            hi,
            DocItem {
                order: 100,
                name: "add".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_const() {
        let hi: DocItem = syn::parse_quote! {
            /// A docstring
            pub const X: usize = 13;
        };
        assert_eq!(
            hi,
            DocItem {
                order: 100,
                name: "X".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_static() {
        let hi: DocItem = syn::parse_quote! {
            /// A docstring
            pub static X: usize = 13;
        };
        assert_eq!(
            hi,
            DocItem {
                order: 100,
                name: "X".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_struct() {
        let hi: DocItem = syn::parse_quote! {
            /// A docstring
            pub struct Foo {}
        };
        assert_eq!(
            hi,
            DocItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_enum() {
        let hi: DocItem = syn::parse_quote! {
            /// A docstring
            pub enum Foo {}
        };
        assert_eq!(
            hi,
            DocItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_union() {
        let hi: DocItem = syn::parse_quote! {
            /// A docstring
            pub union Foo {}
        };
        assert_eq!(
            hi,
            DocItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    fn test_parsing_type() {
        let hi: DocItem = syn::parse_quote! {
            /// A docstring
            pub type Foo = Bar;
        };
        assert_eq!(
            hi,
            DocItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }

    //#[test]
    fn test_parsing_inner() {
        let hi: DocItem = syn::parse_quote! {
            //! A docstring
        };
        assert_eq!(
            hi,
            DocItem {
                order: 100,
                name: "Foo".into(),
                content: "// A docstring".into(),
            }
        );
    }
}
*/
