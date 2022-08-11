#[doc = include_str!("pass-by-pointer-doc.md")]
pub trait PassByPointer: Sized {
    /// Take a value from C as an argument.
    ///
    /// # Safety
    ///
    /// - arg must not be NULL
    /// - arg must be a value returned from Box::into_raw (via return_ptr or ptr_to_arg_out)
    /// - arg becomes invalid and must not be used after this call
    unsafe fn take_from_ptr_arg(arg: *mut Self) -> Self {
        debug_assert!(!arg.is_null());
        // SAFETY: see docstring
        unsafe { *(Box::from_raw(arg)) }
    }

    /// Borrow a value from C as an argument.
    ///
    /// # Safety
    ///
    /// - arg must not be NULL
    /// - *arg must be a valid instance of Self
    /// - arg must be valid for the lifetime assigned by the caller
    /// - arg must not be modified by anything else during that lifetime
    unsafe fn from_ptr_arg_ref<'a>(arg: *const Self) -> &'a Self {
        debug_assert!(!arg.is_null());
        // SAFETY: see docstring
        unsafe { &*arg }
    }

    /// Mutably borrow a value from C as an argument.
    ///
    /// # Safety
    ///
    /// - arg must not be NULL
    /// - *arg must be a valid instance of Self
    /// - arg must be valid for the lifetime assigned by the caller
    /// - arg must not be accessed by anything else during that lifetime
    unsafe fn from_ptr_arg_ref_mut<'a>(arg: *mut Self) -> &'a mut Self {
        debug_assert!(!arg.is_null());
        // SAFETY: see docstring
        unsafe { &mut *arg }
    }

    /// Return a value to C, transferring ownership
    ///
    /// # Safety
    ///
    /// - the caller must ensure that the value is eventually freed
    unsafe fn return_ptr(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }

    /// Return a value to C, transferring ownership, via an "output parameter".
    ///
    /// # Safety
    ///
    /// - the caller must ensure that the value is eventually freed
    /// - arg_out must not be NULL
    /// - arg_out must point to valid, properly aligned memory for a pointer value
    unsafe fn ptr_to_arg_out(self, arg_out: *mut *mut Self) {
        debug_assert!(!arg_out.is_null());
        // SAFETY: see docstring
        unsafe { *arg_out = self.return_ptr() };
    }
}
