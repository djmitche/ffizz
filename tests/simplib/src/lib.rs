ffizz_header::snippet! {
#[ffizz(name="top", order=0)]
/// SimpLib -- addition, simplified.
}

ffizz_header::snippet! {
#[ffizz(name="includes", order=1)]
/// ```c
/// #include <stdint.h>
/// ```
}

#[ffizz_header::item]
/// Add two numbers and return the result.  Overflow will be handled with
/// a panic.
///
/// ```c
/// uint64_t add(uint64_t left, uint64_t right);
/// ```
#[no_mangle]
pub unsafe extern "C" fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(debug_assertions)] // only include this in debug builds
/// Generate the header
pub fn generate_header() -> String {
    ffizz_header::generate()
}
