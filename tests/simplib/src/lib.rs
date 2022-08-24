#[ffizz_header::item]
/// This is my function
/// Another line
///
/// ```c
/// usize add(usize left, usize right);
/// ```
#[no_mangle]
pub unsafe extern "C" fn add(left: usize, right: usize) -> usize {
    left + right
}

pub fn generate_header() -> String {
    ffizz_header::generate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = unsafe { add(2, 2) };
        assert_eq!(result, 4);
    }

    #[test]
    fn test_header() {
        println!("{}", super::generate_header());
        assert_eq!(
            super::generate_header(),
            String::from(
                "// This is my function\n// Another line\n//\n usize add(usize left, usize right);\n"
            )
        );
    }
}
