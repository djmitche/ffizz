use itertools::join;
use linkme::distributed_slice;
use std::cmp::Ordering;

pub use linkme;

/// TODO doc
pub use ffizz_macros::item;

/// A HeaderItem contains an item that should be included in the output C header.  Only the
/// `content` field will actually appear, with the other fields used to ensure a stable order for
/// the items.  `order` is used for coarse-grained ordering, such as putting introductory comments
/// at the top.  For items with equal `order`, `name` is used to sort.
#[derive(Clone)]
pub struct HeaderItem {
    pub order: usize,
    pub name: &'static str,
    pub content: &'static str,
}

/// FFIZZ_HEADER_ITEMS collects HeaderItems using `linkme`.
#[distributed_slice]
pub static FFIZZ_HEADER_ITEMS: [HeaderItem] = [..];

/// Generate the C header for this library.  This sorts all HeaderItems and then combines them
/// into a single string.
pub fn generate() -> String {
    generate_from_vec(FFIZZ_HEADER_ITEMS.iter().collect::<Vec<_>>())
}

/// Inner version of generate that does not operate on a static value.
fn generate_from_vec(mut items: Vec<&'static HeaderItem>) -> String {
    items.sort_by(
        |a: &&'static HeaderItem, b: &&'static HeaderItem| match a.order.cmp(&b.order) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => a.name.cmp(b.name),
            Ordering::Greater => Ordering::Greater,
        },
    );

    // join the items with blank lines
    let mut result = join(items.iter().map(|hi| hi.content.trim()), "\n\n");
    // and ensure a trailing newline
    if items.len() > 0 {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod test {
    #[test]
    fn test_generate_order_by_order() {
        assert_eq!(
            super::generate_from_vec(vec![
                &super::HeaderItem {
                    order: 1,
                    name: "foo",
                    content: "one"
                },
                &super::HeaderItem {
                    order: 3,
                    name: "foo",
                    content: "three"
                },
                &super::HeaderItem {
                    order: 2,
                    name: "foo",
                    content: "two"
                },
            ]),
            String::from("one\n\ntwo\n\nthree\n")
        );
    }

    #[test]
    fn test_generate_order_by_name() {
        assert_eq!(
            super::generate_from_vec(vec![
                &super::HeaderItem {
                    order: 3,
                    name: "bbb",
                    content: "two"
                },
                &super::HeaderItem {
                    order: 3,
                    name: "ccc",
                    content: "three"
                },
                &super::HeaderItem {
                    order: 3,
                    name: "aaa",
                    content: "one"
                },
            ]),
            String::from("one\n\ntwo\n\nthree\n")
        );
    }

    #[test]
    fn test_empty() {
        assert_eq!(super::generate(), String::new());
    }
}
