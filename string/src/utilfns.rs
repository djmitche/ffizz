use crate::{fz_string_t, FzString};
use std::ffi::{CStr, CString};

// These functions are used in downstream creates via the `reexport!` macro, which generates a
// function in that crate, wrapping one of these functions.  As a result, none of these functions
// are `extern "C"`, and all are tagged with `inline(always)` so that they are inlined into the
// downstream crate.
//
// NOTE: if you add a function to this module, also add it to `reexport!` in string/src/macros.rs.

// This type is used in the `reexport!` macro.
#[doc(hidden)]
pub type c_char = libc::c_char;

/// Create a new fz_string_t containing a pointer to the given C string.
///
/// # Safety
///
/// The C string must remain valid and unchanged until after the `fz_string_t` is freed.  It's
/// typically easiest to ensure this by using a static string.
///
/// The resulting `fz_string_t` must be freed.
///
/// ```c
/// fz_string_t fz_string_borrow(const char *);
/// ```
#[inline(always)]
pub unsafe fn fz_string_borrow(cstr: *const c_char) -> fz_string_t {
    debug_assert!(!cstr.is_null());
    // SAFETY:
    //  - cstr is not NULL (promised by caller, verified by assertion)
    //  - cstr's lifetime exceeds that of the fz_string_t (promised by caller)
    //  - cstr contains a valid NUL terminator (promised by caller)
    //  - cstr's content will not change before it is destroyed (promised by caller)
    let cstr: &CStr = unsafe { CStr::from_ptr(cstr) };
    // SAFETY:
    //  - caller promises to free this string
    unsafe { FzString::return_val(FzString::CStr(cstr)) }
}

#[allow(clippy::missing_safety_doc)] // not actually terribly unsafe
/// Create a new, null `fz_string_t`.  Note that this is _not_ the zero value of `fz_string_t`.
///
/// # Safety
///
/// The resulting `fz_string_t` must be freed.
///
/// ```c
/// fz_string_t fz_string_null();
/// ```
#[inline(always)]
pub unsafe fn fz_string_null() -> fz_string_t {
    // SAFETY:
    //  - caller promises to free this string
    unsafe { FzString::return_val(FzString::Null) }
}

/// Create a new `fz_string_t` by cloning the content of the given C string.  The resulting `fz_string_t`
/// is independent of the given string.
///
/// # Safety
///
/// The given pointer must not be NULL.
/// The resulting `fz_string_t` must be freed.
///
/// ```c
/// fz_string_t fz_string_clone(const char *);
/// ```
#[inline(always)]
pub unsafe fn fz_string_clone(cstr: *const c_char) -> fz_string_t {
    debug_assert!(!cstr.is_null());
    // SAFETY:
    //  - cstr is not NULL (promised by caller, verified by assertion)
    //  - cstr's lifetime exceeds that of this function (by C convention)
    //  - cstr contains a valid NUL terminator (promised by caller)
    //  - cstr's content will not change before it is destroyed (by C convention)
    let cstr: &CStr = unsafe { CStr::from_ptr(cstr) };
    let cstring: CString = cstr.into();
    // SAFETY:
    //  - caller promises to free this string
    unsafe { FzString::return_val(FzString::CString(cstring)) }
}

/// Create a new `fz_string_t` containing the given string with the given length. This allows creation
/// of strings containing embedded NUL characters.  As with `fz_string_clone`, the resulting
/// `fz_string_t` is independent of the passed buffer.
///
/// The given length should _not_ include any NUL terminator.  The given length must be less than
/// half the maximum value of usize.
///
/// # Safety
///
/// The given pointer must not be NULL.
/// The resulting `fz_string_t` must be freed.
///
/// ```c
/// fz_string_t fz_string_clone_with_len(const char *ptr, usize len);
/// ```
#[inline(always)]
pub unsafe fn fz_string_clone_with_len(buf: *const c_char, len: usize) -> fz_string_t {
    debug_assert!(!buf.is_null());
    debug_assert!(len < isize::MAX as usize);
    // SAFETY:
    //  - buf is valid for len bytes (by C convention)
    //  - (no alignment requirements for a byte slice)
    //  - content of buf will not be mutated during the lifetime of this slice (lifetime
    //    does not outlive this function call)
    //  - the length of the buffer is less than isize::MAX (promised by caller)
    let slice = unsafe { std::slice::from_raw_parts(buf as *const u8, len) };

    // allocate and copy into Rust-controlled memory
    let vec = slice.to_vec();

    // SAFETY:
    //  - caller promises to free this string
    unsafe { FzString::return_val(FzString::Bytes(vec)) }
}

/// Get the content of the string as a regular C string.
///
/// A string contianing NUL bytes will result in a NULL return value.  In general, prefer
/// `fz_string_content_with_len` except when it's certain that the string is NUL-free.
///
/// The Null variant also results in a NULL return value.
///
/// This function takes the `fz_string_t` by pointer because it may be modified in-place to add a NUL
/// terminator.  The pointer must not be NULL.
///
/// # Safety
///
/// The returned string is "borrowed" and remains valid only until the `fz_string_t` is freed or
/// passed to any other API function.
///
/// ```c
/// const char *fz_string_content(fz_string_t *);
/// ```
#[inline(always)]
pub unsafe fn fz_string_content(fzstr: *mut fz_string_t) -> *const c_char {
    // SAFETY;
    //  - fzstr is not NULL (promised by caller, verified)
    //  - *fzstr is valid (promised by caller)
    //  - *fzstr is not accessed concurrently (single-threaded)
    unsafe {
        FzString::with_ref_mut(fzstr, |fzstr| match fzstr.as_cstr() {
            // SAFETY:
            //  - implied lifetime here is FzString's lifetime; valid until another mutable
            //    reference is made (see docstring)
            Ok(Some(cstr)) => cstr.as_ptr(),
            _ => std::ptr::null(),
        })
    }
}

/// Get the content of the string as a pointer and length.
///
/// This function can return any string, even one including NUL bytes or invalid UTF-8.
/// If the FzString is the Null variant, this returns NULL and the length is set to zero.
///
/// # Safety
///
/// The returned string is "borrowed" and remains valid only until the `fz_string_t` is freed or
/// passed to any other API function.
///
/// ```c
/// const char *fz_string_content_with_len(const struct fz_string_t *, len_out *usize);
/// ```
#[inline(always)]
pub unsafe fn fz_string_content_with_len(
    fzstr: *mut fz_string_t,
    len_out: *mut usize,
) -> *const c_char {
    // SAFETY;
    //  - fzstr is not NULL (promised by caller)
    //  - *fzstr is valid (promised by caller)
    //  - *fzstr is not accessed concurrently (single-threaded)
    unsafe {
        FzString::with_ref(fzstr, |fzstr| {
            let bytes = match fzstr.as_bytes() {
                Some(bytes) => bytes,
                None => {
                    // SAFETY:
                    //  - len_out is not NULL (promised by caller)
                    //  - len_out points to valid memory (promised by caller)
                    //  - len_out is properly aligned (C convention)
                    unsafe {
                        *len_out = 0;
                    }
                    return std::ptr::null();
                }
            };

            // SAFETY:
            //  - len_out is not NULL (promised by caller)
            //  - len_out points to valid memory (promised by caller)
            //  - len_out is properly aligned (C convention)
            unsafe {
                *len_out = bytes.len();
            }
            bytes.as_ptr() as *const c_char
        })
    }
}

#[allow(clippy::missing_safety_doc)] // NULL pointer is OK so not actually unsafe
/// Determine whether the given `fz_string_t` is a Null variant.
///
/// ```c
/// bool fz_string_is_null(fz_string_t *);
/// ```
#[inline(always)]
pub unsafe fn fz_string_is_null(fzstr: *const fz_string_t) -> bool {
    unsafe { FzString::with_ref(fzstr, |fzstr| fzstr.is_null()) }
}

/// Free a `fz_string_t`.
///
/// # Safety
///
/// The string must not be used after this function returns, and must not be freed more than once.
/// It is safe to free Null-variant strings.
///
/// ```c
/// fz_string_free(fz_string_t *);
/// ```
#[inline(always)]
pub unsafe fn fz_string_free(fzstr: *mut fz_string_t) {
    // SAFETY:
    //  - fzstr is not NULL (promised by caller)
    //  - caller will not use this value after return
    drop(unsafe { FzString::take_ptr(fzstr) });
}

#[cfg(test)]
mod test {
    use super::*;

    const INVALID_UTF8: &[u8] = b"abc\xf0\x28\x8c\x28";

    #[test]
    fn borrow() {
        let s = CString::new("hello!").unwrap();
        let ptr = unsafe { s.as_ptr() };

        let mut fzstr = unsafe { fz_string_borrow(ptr) };
        assert!(unsafe { !fz_string_is_null(&fzstr as *const fz_string_t) });

        let content = unsafe { CStr::from_ptr(fz_string_content(&mut fzstr as *mut fz_string_t)) };
        assert_eq!(content.to_str().unwrap(), "hello!");

        drop(s); // make sure s lasts long enough!

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    #[test]
    fn borrow_invalid_utf8() {
        let s = CString::new(INVALID_UTF8).unwrap();
        let ptr = unsafe { s.as_ptr() };

        let mut fzstr = unsafe { fz_string_borrow(ptr) };
        assert!(unsafe { !fz_string_is_null(&fzstr as *const fz_string_t) });

        let mut len: usize = 0;
        let ptr = unsafe {
            fz_string_content_with_len(&mut fzstr as *mut fz_string_t, &mut len as *mut usize)
        };
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        assert_eq!(slice, INVALID_UTF8);

        drop(s); // make sure s lasts long enough!

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    #[test]
    fn clone() {
        let s = CString::new("hello!").unwrap();
        let ptr = unsafe { s.as_ptr() };

        let mut fzstr = unsafe { fz_string_clone(ptr) };
        assert!(unsafe { !fz_string_is_null(&fzstr as *const fz_string_t) });

        drop(s); // fzstr contains a clone of s, so deallocate

        let content = unsafe { CStr::from_ptr(fz_string_content(&mut fzstr as *mut fz_string_t)) };
        assert_eq!(content.to_str().unwrap(), "hello!");

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    #[test]
    fn null_and_is_null() {
        let mut fzstr = unsafe { fz_string_null() };
        assert!(unsafe { fz_string_is_null(&fzstr as *const fz_string_t) });

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    #[test]
    fn null_ptr_is_null() {
        let mut fzstr = unsafe { fz_string_null() };
        assert!(unsafe { fz_string_is_null(std::ptr::null()) });

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    #[test]
    fn clone_invalid_utf8() {
        let s = CString::new(INVALID_UTF8).unwrap();
        let ptr = unsafe { s.as_ptr() };

        let mut fzstr = unsafe { fz_string_clone(ptr) };
        assert!(unsafe { !fz_string_is_null(&fzstr as *const fz_string_t) });

        drop(s); // fzstr contains a clone of s, so deallocate

        let mut len: usize = 0;
        let ptr = unsafe {
            fz_string_content_with_len(&mut fzstr as *mut fz_string_t, &mut len as *mut usize)
        };
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        assert_eq!(slice, INVALID_UTF8);

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    #[test]
    fn clone_with_len() {
        let s = CString::new("ABCDEFGH").unwrap();
        let ptr = unsafe { s.as_ptr() };

        let mut fzstr = unsafe { fz_string_clone_with_len(ptr, 4) };
        assert!(unsafe { !fz_string_is_null(&fzstr as *const fz_string_t) });

        drop(s); // fzstr contains a clone of s, so deallocate

        let content = unsafe { CStr::from_ptr(fz_string_content(&mut fzstr as *mut fz_string_t)) };
        assert_eq!(content.to_str().unwrap(), "ABCD"); // only 4 bytes

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    #[test]
    fn clone_with_len_invalid_utf8() {
        let s = CString::new(INVALID_UTF8).unwrap();
        let ptr = unsafe { s.as_ptr() };

        let mut fzstr = unsafe { fz_string_clone_with_len(ptr, 4) };
        assert!(unsafe { !fz_string_is_null(&fzstr as *const fz_string_t) });

        drop(s); // fzstr contains a clone of s, so deallocate

        let mut len: usize = 0;
        let ptr = unsafe {
            fz_string_content_with_len(&mut fzstr as *mut fz_string_t, &mut len as *mut usize)
        };
        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        assert_eq!(slice, &INVALID_UTF8[..4]); // only 4 bytes

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    // (fz_string_content's normal operation is tested above)

    #[test]
    fn content_nul_bytes() {
        let s = String::from("hello \0 NUL byte");
        let ptr = unsafe { s.as_ptr() } as *mut c_char;

        let mut fzstr = unsafe { fz_string_clone_with_len(ptr, s.len()) };
        assert!(unsafe { !fz_string_is_null(&fzstr as *const fz_string_t) });

        let ptr = unsafe { fz_string_content(&mut fzstr as *mut fz_string_t) };

        // could not return a string because of the embedded NUL byte
        assert!(ptr.is_null());

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    #[test]
    fn content_null_ptr() {
        let ptr = unsafe { fz_string_content(std::ptr::null_mut()) };
        assert!(ptr.is_null());
    }

    #[test]
    fn content_with_len_nul_bytes() {
        let s = String::from("hello \0 NUL byte");
        let ptr = unsafe { s.as_ptr() } as *mut c_char;

        let mut fzstr = unsafe { fz_string_clone_with_len(ptr, s.len()) };
        assert!(unsafe { !fz_string_is_null(&fzstr as *const fz_string_t) });

        let mut len: usize = 0;
        let ptr = unsafe {
            fz_string_content_with_len(&mut fzstr as *mut fz_string_t, &mut len as *mut usize)
        };

        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
        let s = std::str::from_utf8(slice).unwrap();
        assert_eq!(s, "hello \0 NUL byte");

        unsafe { fz_string_free(&mut fzstr as *mut fz_string_t) };
    }

    #[test]
    fn content_with_len_null_ptr() {
        let mut len: usize = 9999;
        let ptr =
            unsafe { fz_string_content_with_len(std::ptr::null_mut(), &mut len as *mut usize) };
        assert!(ptr.is_null());
        assert_eq!(len, 0);
    }

    // (fz_string_free is tested above)
}
