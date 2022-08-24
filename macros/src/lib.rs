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
        let item: syn::Item = input.parse()?;
        match item {
            syn::Item::Fn(itemfn) => {
                // extract any docstring
                let mut doc = vec![];
                for attr in itemfn.attrs {
                    if let Ok(syn::Meta::NameValue(nv)) = attr.parse_meta() {
                        if nv.path.is_ident("doc") {
                            if let syn::Lit::Str(s) = nv.lit {
                                doc.push(s.value());
                            }
                        }
                    }
                }

                // extract the function name
                let name = itemfn.sig.ident.to_string();

                if doc.len() == 0 {
                    return Result::Err(Error::new(
                        Span::call_site(),
                        format!("{} does not have a docstring", name),
                    ));
                }

                Ok(parse_docstring(name, doc))
            }
            _ => Result::Err(Error::new_spanned(
                item,
                "cannot determine header content from this item",
            )),
        }
    }
}

impl HeaderItem {
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

#[proc_macro_attribute]
pub fn function(attr: TokenStream, mut item: TokenStream) -> TokenStream {
    let parsed = {
        let item = item.clone();
        syn::parse_macro_input!(item as HeaderItem)
    };
    item.extend(TokenStream::from(parsed.into_tokens()));
    item
}

/// parse a docstring to extract C declarations and comments.
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
}
