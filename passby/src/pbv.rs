/// This trait supports passing data to Rust by value.
///
/// Pass-by-values implies that values are copyable, via assignment in C, so this
/// trait is typically used to represent Copy values, and in particular values that
/// do not contain pointers.
///
/// The Rust and C types may differ, with [`PassByValue::from_ctype`] and [`PassByValue::into_ctype`]
/// converting between them.  These typically provide some simple conversion between a C-style
/// data structure and a more ergonomic Rust type.
pub trait PassByValue: Sized {
    /// The C representation of this type.
    type CType: Sized;

    /// Convert a C value to a Rust value.
    ///
    /// # Safety
    ///
    /// The implementation of this method assumes that `cval` is a valid instance of Self::CType.
    unsafe fn from_ctype(cval: Self::CType) -> Self;

    /// Convert a Rust value to a C value.
    fn into_ctype(self) -> Self::CType;

    /// Copy a value from C as an argument.
    ///
    /// # Safety
    ///
    /// - `self` must be a valid instance of the C type.  This is typically ensured either by
    ///   requiring that C code not modify it, or by defining the valid values in C comments.
    unsafe fn val_from_arg(arg: Self::CType) -> Self {
        // SAFETY:
        //  - arg is a valid CType (see docstring)
        unsafe { Self::from_ctype(arg) }
    }

    /// Return a value to C
    ///
    /// # Safety
    ///
    /// - if the value is allocated, the caller must ensure that the value is eventually freed
    unsafe fn return_val(self) -> Self::CType {
        self.into_ctype()
    }

    /// Return a value to C, via an "output parameter".
    ///
    /// This is common in functions returning a new value along with some success indication.
    ///
    /// # Safety
    ///
    /// - `arg_out` must not be NULL and must be properly aligned and pointing to valid memory
    ///   of the size of CType.
    unsafe fn val_to_arg_out(self, arg_out: *mut Self::CType) {
        debug_assert!(!arg_out.is_null());
        // SAFETY:
        //  - arg_out is not NULL (see docstring)
        //  - arg_out is properly aligned and points to valid memory (see docstring)
        unsafe { *arg_out = self.into_ctype() };
    }
}
