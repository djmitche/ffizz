use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::Error;

struct HeaderItem {
    order: usize,
    name: String,
    content: String,
}

#[proc_macro_attribute]
pub fn function(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed = {
        let item = item.clone();
        syn::parse_macro_input!(item as syn::ItemFn)
    };
    function2(TokenStream2::from(attr), TokenStream2::from(item), parsed).into()
}

fn function2(attr: TokenStream2, existing: TokenStream2, itemfn: syn::ItemFn) -> TokenStream2 {
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
        return with_error(existing, format!("{} does not have a docstring", name));
    }

    let header_item = match parse_docstring(name, doc) {
        Ok(header_item) => header_item,
        Err(msg) => return with_error(existing, msg),
    };

    with_header_item(existing, header_item)
}

/// parse a docstring to extract C declarations and comments.
fn parse_docstring(name: String, doc: Vec<String>) -> Result<HeaderItem, String> {
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

    Ok(HeaderItem {
        order: 100, // default
        name,
        content: itertools::join(content, "\n"),
    })
}

/// Return a token stream containing the existing content as well as a compile_error!() invocation
/// with the given message
fn with_error<T: std::fmt::Display>(mut existing: TokenStream2, error: T) -> TokenStream2 {
    let addition = Error::new(Span::call_site(), error).to_compile_error();
    existing.extend(addition);
    existing
}

/// Return a token stream containing the existing content as well as a declaration of the given
/// header item
fn with_header_item(mut existing: TokenStream2, header: HeaderItem) -> TokenStream2 {
    // NOTE: we can't use `#[::linkme::distributed_slice(HEADER_ITEMS)]` here because it assume
    // that `linkme` is in the dependencies of the crate being compiled.  We do not want to "leak"
    // the dependency on linkme, so this is manually expanded.
    let HeaderItem {
        order,
        name,
        content,
    } = header;
    let addition: TokenStream2 = quote! {
        #[used]
        #[link_section = "linkme_HEADER_ITEMS"]
        static HEADER_ITEM: ::ffizz_header::HeaderItem = ::ffizz_header::HeaderItem {
            order: #order,
            name: #name,
            content: #content,
        };
    }
    .into();
    existing.extend(addition);
    existing
}

// TODO: test
