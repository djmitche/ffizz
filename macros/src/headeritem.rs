use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Error, Result};

/// The default order for a header item.
const DEFAULT_ORDER: usize = 100;

/// HeaderItem is a proc-macro-execution-time version of the HeaderItem object these macros will
/// insert into the Rust code.
#[derive(Debug, PartialEq)]
pub(crate) struct HeaderItem {
    pub(crate) order: usize,
    pub(crate) name: String,
    pub(crate) content: String,
}

impl HeaderItem {
    /// Create a HeaderItem, given a name and a vec of its attributes.  All ffizz_header-specific
    /// attributes are removed from attrs, and all docstrings are parsed into C header content.
    pub(crate) fn from_attrs(name: String, attrs: &mut Vec<syn::Attribute>) -> Result<Self> {
        let (doc, override_name, override_order) = Self::parse_attrs(attrs)?;
        let content = Self::parse_content(doc);
        Ok(Self {
            name: override_name.unwrap_or(name),
            order: override_order.unwrap_or(DEFAULT_ORDER),
            content,
        })
    }

    /// Parse a vec of attributes, extracting docstrings and ffizz attributes (name and header).
    /// Any ffizz attributes are removed from the given vector.
    ///
    /// Returns the docstrings, the name property (if found), and the order (if found)
    pub(crate) fn parse_attrs(
        attrs: &mut Vec<syn::Attribute>,
    ) -> Result<(Vec<String>, Option<String>, Option<usize>)> {
        let mut order = None;
        let mut name = None;

        let mut doc: Vec<String> = vec![];
        let mut kept_attrs = vec![];
        for attr in attrs.drain(..) {
            let mut keep_attr = true;
            match attr.parse_meta() {
                // docstrings are represented as #[doc = r"..."]
                Ok(syn::Meta::NameValue(nv)) => {
                    if nv.path.is_ident("doc") {
                        if let syn::Lit::Str(s) = nv.lit {
                            let s = s.value();
                            doc.extend(Self::parse_docstring_attr(s));
                        }
                    }
                }
                Ok(syn::Meta::List(metalist)) => {
                    if metalist.path.is_ident("ffizz") {
                        keep_attr = false;
                        for elt in metalist.nested {
                            let mut ok = false;
                            if let syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) = elt {
                                if nv.path.is_ident("name") {
                                    if let syn::Lit::Str(s) = nv.lit {
                                        name = Some(s.value());
                                        ok = true;
                                    }
                                } else if nv.path.is_ident("order") {
                                    if let syn::Lit::Int(i) = nv.lit {
                                        if let Ok(i) = i.base10_parse::<usize>() {
                                            order = Some(i);
                                            ok = true;
                                        }
                                    }
                                }
                            }
                            if !ok {
                                return Err(Error::new_spanned(
                                    attr,
                                    "Valid #[fizz(..)] attribute properties here are name=\"..\" and order=.."
                                ));
                            }
                        }
                    }
                }
                _ => {
                    // ignore (and keep) any other attributes
                }
            }
            if keep_attr {
                kept_attrs.push(attr);
            }
        }
        *attrs = kept_attrs;

        Ok((doc, name, order))
    }

    /// Parse a docstring attribute value into an array of docstring lines, accounting for
    /// the peculiar ways we receive these from the parser.  The goal here is to capture
    /// the user's intended text, without any indentation or `*` prefixes.
    fn parse_docstring_attr(s: String) -> Vec<String> {
        // We get everything but the comment characters, including whitespace.
        //  - For `/// foo`, we will get " foo".
        //  - For `/** \n    * foo */`, we will get " \n    * foo ".

        // For simplicity, we assume the docstring is either using `///` or
        // included in one big `/** .. */` comment.
        if s.contains('\n') {
            // /** ... */ - style

            let mut lines: Vec<_> = s.split('\n').collect();

            fn is_boring(line: Option<&&str>) -> bool {
                line.map(|line| line.chars().all(|c| c.is_whitespace() || c == '*'))
                    .unwrap_or(false)
            }

            // if the first line is boring, we can drop it.  This is the case when the `/**`
            // is alone on the first line.
            if is_boring(lines.first()) {
                lines.remove(0);
            }

            // if the last line is boring, it will mess up the prefix determination, so remove it.
            // A non-boring last line occurs when the comment is terminated with `*/` on the same
            // line.
            if is_boring(lines.last()) {
                lines.pop();
            }

            // now any remaining lines after the first probably have a common
            // prefix of whitespace and `*`.  Guess that from the last line.
            let prefix = lines
                .last()
                .map(|first_line| {
                    let offset = first_line
                        .find(|c: char| !(c.is_whitespace() || c == '*'))
                        .unwrap_or(first_line.len());
                    first_line[..offset].to_string()
                })
                .unwrap_or_else(String::new);

            // and remove it from all lines where it appears
            let lines: Vec<String> = lines
                .iter()
                .map(|line| {
                    if line.starts_with(&prefix) {
                        line[prefix.len()..].to_string()
                    } else {
                        line.to_string()
                    }
                })
                .collect();

            lines
        } else {
            // /// - style

            // strip a single leading space, if it exists.
            if let Some(stripped) = s.strip_prefix(' ') {
                vec![stripped.to_string()]
            } else {
                vec![s]
            }
        }
    }

    /// Parse a docstring, presented as a vec of lines, to extract C declarations and comments.
    pub(crate) fn parse_content(doc: Vec<String>) -> String {
        let mut content = vec![];
        let mut in_decl = false;
        let mut strip_new_blank_comments = true;

        /// strip trailing blank comment lines
        fn strip_trailing_blank_comments(lines: &mut Vec<String>) {
            while let Some(line) = lines.last() {
                if line == "//" {
                    lines.pop();
                } else {
                    break;
                }
            }
        }

        for line in doc {
            if in_decl {
                if line.trim() == "```" {
                    in_decl = false;
                    strip_new_blank_comments = true;
                    continue;
                }
                content.push(line);
            } else {
                if strip_new_blank_comments && line.is_empty() {
                    continue;
                }
                if line.trim() == "```c" {
                    in_decl = true;
                    strip_trailing_blank_comments(&mut content);
                    continue;
                }
                if !line.is_empty() {
                    content.push(format!("// {}", line));
                } else {
                    content.push("//".to_string());
                }
                strip_new_blank_comments = false;
            }
        }

        strip_trailing_blank_comments(&mut content);

        itertools::join(content, "\n")
    }

    /// Write the content of this HeaderItem into a TokenStream such that the resulting binary will
    /// include the HeaderItem in its `::ffizz_header::FFIZZ_HEADER_ITEMS` array.
    pub(crate) fn to_tokens(&self, tokens: &mut TokenStream2) {
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
}

#[cfg(test)]
mod test {
    use super::*;
    use syn::parse::{Parse, ParseStream};
    use syn::parse_quote;

    struct Attrs(Vec<syn::Attribute>);

    impl Parse for Attrs {
        fn parse(input: ParseStream) -> Result<Self> {
            let attrs = input.call(syn::Attribute::parse_outer)?;
            Ok(Attrs(attrs))
        }
    }

    #[test]
    fn parse_attrs_simple() {
        let mut attrs: Attrs = parse_quote! {
            /// aaa
            /// bbb
        };
        let (doc, name, order) = HeaderItem::parse_attrs(&mut attrs.0).unwrap();
        assert_eq!(order, None);
        assert_eq!(name, None);
        assert_eq!(doc, vec!["aaa", "bbb"]);
    }

    #[test]
    fn parse_attrs_multiline() {
        let mut attrs: Attrs = parse_quote! {
            /**
             * aaa
             * bbb
             */
        };
        let (doc, name, order) = HeaderItem::parse_attrs(&mut attrs.0).unwrap();
        assert_eq!(order, None);
        assert_eq!(name, None);
        assert_eq!(doc, vec!["aaa", "bbb"]);
    }

    #[test]
    fn parse_attrs_single_override_attr() {
        let mut attrs: Attrs = parse_quote! {
            /// aaa
            #[ffizz(name="override")]
            /// bbb
        };
        let (doc, name, order) = HeaderItem::parse_attrs(&mut attrs.0).unwrap();
        assert_eq!(order, None);
        assert_eq!(name, Some(String::from("override")));
        assert_eq!(doc, vec!["aaa", "bbb"]);
        // check that the #[ffizz(..)] attributes were stripped
        assert_eq!(attrs.0.len(), 2);
    }

    #[test]
    fn parse_attrs_multi_override_attr() {
        let mut attrs: Attrs = parse_quote! {
            #[ffizz(name="not seen")]
            /// aaa
            #[ffizz(name="override")]
            #[ffizz(order=13)]
            /// bbb
        };
        let (doc, name, order) = HeaderItem::parse_attrs(&mut attrs.0).unwrap();
        assert_eq!(order, Some(13));
        assert_eq!(name, Some(String::from("override")));
        assert_eq!(doc, vec!["aaa", "bbb"]);
        // check that the #[ffizz(..)] attributes were stripped
        assert_eq!(attrs.0.len(), 2);
    }

    #[test]
    fn parse_attrs_name_order_same_attr() {
        let mut attrs: Attrs = parse_quote! {
            #[ffizz(name="override", order=13)]
            /// aaa
            /// bbb
        };
        let (doc, name, order) = HeaderItem::parse_attrs(&mut attrs.0).unwrap();
        assert_eq!(order, Some(13));
        assert_eq!(name, Some(String::from("override")));
        assert_eq!(doc, vec!["aaa", "bbb"]);
        // check that the #[ffizz(..)] attributes were stripped
        assert_eq!(attrs.0.len(), 2);
    }

    #[test]
    fn parse_attrs_invalid_ffizz_attr() {
        let mut attrs: Attrs = parse_quote! {
            #[ffizz(blergh="uhoh", snars=13)]
            /// aaa
            /// bbb
        };
        assert!(HeaderItem::parse_attrs(&mut attrs.0).is_err());
    }

    fn multiline(s: &'static str) -> String {
        // strip `/**` and `*/`.
        s[3..s.len() - 2].to_string()
    }

    #[test]
    fn parse_doc_attr_multiline_1() {
        assert_eq!(
            HeaderItem::parse_docstring_attr(multiline(
                "/**
                  * hello
                  */"
            )),
            vec!["hello".to_string()],
        )
    }

    #[test]
    fn parse_doc_attr_multiline_2() {
        assert_eq!(
            HeaderItem::parse_docstring_attr(multiline(
                "/** hello
                  */"
            )),
            vec!["hello".to_string()],
        )
    }

    #[test]
    fn parse_doc_attr_multiline_3() {
        assert_eq!(
            HeaderItem::parse_docstring_attr(multiline(
                "/**
                  */"
            )),
            Vec::<String>::new(),
        )
    }

    #[test]
    fn parse_doc_attr_multiline_4() {
        assert_eq!(
            HeaderItem::parse_docstring_attr(multiline(
                "/**
                  * two
                  * lines
                  */"
            )),
            vec!["two", "lines"],
        )
    }

    #[test]
    fn parse_doc_attr_multiline_5() {
        assert_eq!(
            HeaderItem::parse_docstring_attr(multiline(
                "/**
                  * three
                  *   indented
                  * lines
                  */"
            )),
            vec!["three", "  indented", "lines"],
        )
    }

    #[test]
    fn parse_doc_attr_single_line() {
        assert_eq!(HeaderItem::parse_docstring_attr(" foo".into()), vec!["foo"],)
    }

    #[test]
    fn parse_doc_attr_single_line_empty() {
        assert_eq!(HeaderItem::parse_docstring_attr("".into()), vec![""],)
    }

    #[test]
    fn parse_content_just_text() {
        assert_eq!(
            HeaderItem::parse_content(vec!["some".to_string(), "content".to_string()]),
            "// some\n// content".to_string()
        );
    }

    #[test]
    fn parse_content_single_decl() {
        assert_eq!(
            HeaderItem::parse_content(vec![
                "intro".to_string(),
                "```c".to_string(),
                "void foo(void);".to_string(),
                "```".to_string(),
                "suffix".to_string(),
            ]),
            "// intro\nvoid foo(void);\n// suffix".to_string()
        );
    }

    #[test]
    fn parse_content_empty_lines() {
        assert_eq!(
            HeaderItem::parse_content(vec![
                "".to_string(),
                "intro".to_string(),
                "".to_string(),
                "suffix".to_string(),
                "".to_string(),
            ]),
            "// intro\n//\n// suffix".to_string()
        );
    }

    #[test]
    fn parse_content_multi_decl() {
        assert_eq!(
            HeaderItem::parse_content(vec![
                "aaa".to_string(),
                "".to_string(),
                "```c".to_string(),
                "void foo(void);".to_string(),
                "```".to_string(),
                "".to_string(),
                "bbb".to_string(),
                "".to_string(),
                "```c".to_string(),
                "void bar(void);".to_string(),
                "```".to_string(),
                "".to_string(),
            ]),
            "// aaa\nvoid foo(void);\n// bbb\nvoid bar(void);".to_string()
        );
    }
}
