/// Re-export a `fz_string_t` utility function in your own crate.
///
/// For each utility function, this can be written either as
///
/// ```ignore
/// ffizz_string::reexport!(fz_string_free);
/// ```
/// or, to rename the function,
/// ```ignore
/// ffizz_string::reexport!(fz_string_free as my_crate_string_free);
/// ```
///
/// It is still up to you to include project-specific documentation and declaration, typically
/// using `#ffizz_header::snippet!`, due to limitations in the Rust parser around docstrings and
/// macros. For example:
///
/// ```ignore
/// ffizz_snippet!{
///     #[ffizz(name="my_crate_string_free")]
///     /// Free a string ...
///     /// ```c
///     /// EXTERN_C void my_crate_string_free(*my_crate_string);
///     /// ```
/// }
/// ffizz_string::reexport!(fz_string_free as my_crate_string_free);
/// ```
#[macro_export]
macro_rules! reexport(
    // all functions in src/string/utilfns.rs should be reflected here.
    { fz_string_borrow } => { reexport!(fz_string_borrow as fz_string_borrow); };
    { fz_string_borrow as $name:ident } => {
        #[no_mangle]
        #[allow(unsafe_op_in_unsafe_fn)]
        pub unsafe extern "C" fn $name(cstr: *const $crate::c_char) -> $crate::fz_string_t {
            $crate::fz_string_borrow(cstr)
        }
    };
    { fz_string_null } => { reexport!(fz_string_null as fz_string_null); };
    { fz_string_null as $name:ident } => {
        #[no_mangle]
        #[allow(unsafe_op_in_unsafe_fn)]
        pub unsafe extern "C" fn $name() -> $crate::fz_string_t {
            $crate::fz_string_null()
        }
    };
    { fz_string_clone } => { reexport!(fz_string_clone as fz_string_clone); };
    { fz_string_clone as $name:ident } => {
        #[no_mangle]
        #[allow(unsafe_op_in_unsafe_fn)]
        pub unsafe extern "C" fn $name(cstr: *const $crate::c_char) -> $crate::fz_string_t {
            $crate::fz_string_clone(cstr)
        }
    };
    { fz_string_clone_with_len } => { reexport!(fz_string_clone_with_len as fz_string_clone_with_len); };
    { fz_string_clone_with_len as $name:ident } => {
        #[no_mangle]
        #[allow(unsafe_op_in_unsafe_fn)]
        pub unsafe extern "C" fn $name(cstr: *const $crate::c_char, len: usize) -> $crate::fz_string_t {
            $crate::fz_string_clone_with_len(cstr, len)
        }
    };
    { fz_string_content } => { reexport!(fz_string_content as fz_string_content); };
    { fz_string_content as $name:ident } => {
        #[no_mangle]
        #[allow(unsafe_op_in_unsafe_fn)]
        pub unsafe extern "C" fn $name(fzstr: *mut $crate::fz_string_t) -> *const $crate::c_char {
            $crate::fz_string_content(fzstr)
        }
    };
    { fz_string_content_with_len } => { reexport!(fz_string_content_with_len as fz_string_content_with_len); };
    { fz_string_content_with_len as $name:ident } => {
        #[no_mangle]
        #[allow(unsafe_op_in_unsafe_fn)]
        pub unsafe extern "C" fn $name(fzstr: *mut $crate::fz_string_t, len_out: *mut usize) -> *const $crate::c_char {
            $crate::fz_string_content_with_len(fzstr, len_out)
        }
    };
    { fz_string_is_null } => { reexport!(fz_string_is_null as fz_string_is_null); };
    { fz_string_is_null as $name:ident } => {
        #[no_mangle]
        #[allow(unsafe_op_in_unsafe_fn)]
        pub unsafe extern "C" fn $name(fzstr: *const $crate::fz_string_t) -> bool {
            $crate::fz_string_is_null(fzstr)
        }
    };
    { fz_string_free } => { reexport!(fz_string_free as fz_string_free); };
    { fz_string_free as $name:ident } => {
        #[no_mangle]
        #[allow(unsafe_op_in_unsafe_fn)]
        pub unsafe extern "C" fn $name(fzstr: *mut $crate::fz_string_t) {
            $crate::fz_string_free(fzstr)
        }
    };
);

#[cfg(test)]
mod test {
    use std::mem::MaybeUninit;

    reexport!(fz_string_borrow);
    reexport!(fz_string_null);
    reexport!(fz_string_clone);
    reexport!(fz_string_clone_with_len);
    reexport!(fz_string_content);
    reexport!(fz_string_content_with_len);
    reexport!(fz_string_is_null as is_null);
    reexport!(fz_string_free as free_willy);

    #[test]
    fn test() {
        // This doesn't test all of the variants, as they are formulaic and the macro invocations
        // above will catch any differences in the function signatures.

        // SAFETY: we will free this value eventually
        let mut s = MaybeUninit::new(unsafe { fz_string_null() });
        // SAFETY: s contains a valid fz_string_t.
        assert!(unsafe { is_null(s.as_ptr()) });
        // SAFETY: s contains a valid fz_string_t. It is uninitialized
        // after this call and not used again.
        unsafe { free_willy(s.as_mut_ptr()) }
    }
}
