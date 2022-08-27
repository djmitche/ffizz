use crate::headeritem::HeaderItem;
use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::parse::{Error, Parse, ParseStream, Result};

/// DocItem is the result of parsing a "bare" docstring in a `snippet! { .. }` macro invocation,
/// with a header_item constructed from the docstrings and any ffizz-related attributes.
#[derive(Debug, PartialEq)]
pub(crate) struct Snippet {
    header_item: HeaderItem,
}

impl Parse for Snippet {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut attrs = input.call(syn::Attribute::parse_outer)?;
        let header_item = HeaderItem::from_attrs(String::new(), &mut attrs)?;
        if header_item.name.is_empty() {
            return Err(Error::new(
                Span::call_site(),
                "snippet! requires a name (`#[ffizz(name=\"..\")]`)",
            ));
        }
        Ok(Snippet { header_item })
    }
}

impl Snippet {
    /// Convert this DocItem into a TokenStream that will include it in the built binary.
    pub(crate) fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.header_item.to_tokens(tokens);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let di: Snippet = syn::parse_quote! {
            #[ffizz(name="intro")]
            /// A docstring
        };
        assert_eq!(
            di.header_item,
            HeaderItem {
                order: 100,
                name: "intro".into(),
                content: "// A docstring".into(),
            }
        );
    }

    #[test]
    #[should_panic]
    fn test_parse_no_name() {
        let _: Snippet = syn::parse_quote! {
            /// A docstring
        };
    }
}
