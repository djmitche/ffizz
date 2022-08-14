/// This trait supports values passed to Rust by pointer.
/// These values are represented as in C, and always handled as pointers.
///
/// Typically PassByPointer is used to model objects managed entirely by Rust.  These are
/// represented in the C API by a pointer to an opaque struct, with "new" and "free" functions
/// handling creation and destruction.
pub trait PassByPointer: Sized {
    /// Take a value from C as an argument.
    ///
    /// This function is typically used to handle arguments passed from C, but because it takes
    /// ownership of the passed value, while leaving the C code with a pointer, it can lead to
    /// use-after-free errors if not used carefully.  It is most common in "free" functions,
    /// but can also be used in contexts where it is clearl that the called function consumes
    /// the value.  For example, a database connections's `execute` method might reasonably
    /// consume a query argument.
    ///
    /// ```c
    /// db_query_t q = db_query_new();
    /// db_query_set_filter(q, "x = 10");
    /// db_query_add_column(q, "y");
    /// db_result_t res = db_execute(db, q);
    /// ```
    ///
    /// Here it's natural to assume (but should also be documented) that the `db_execute`
    /// function takes ownership of the query.
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
    /// This represents an immutable (shared) borrow.  Use `from_ptr_arg_ref_mut` for
    /// mutable (exclusive) borrows.  The safety requirements of the two methods differ
    /// slightly: this method requires that the value not be concurrently modified, while
    /// `from_ptr_arg_ref_mut` requires that the value not be accessed at all.
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
    /// Because this is a mutable (exclusive) reference, the C caller must ensure
    /// that no other threads _access_ the contained value during the lifetime of
    /// this reference.  This includes read-only access
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

    /// Return a value to C, transferring ownership.
    ///
    /// This method is most often used in constructors, to return the built value.
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
    ///
    /// # Example
    ///
    /// This method is useful for constructors that return multiple values.
    ///
    /// ```
    /// # use ffizz_passby::PassByPointer;
    /// # struct Endpoint { }
    /// # fn pipeline() -> (Endpoint, Endpoint) { todo!() }
    /// # #[allow(non_camel_case_types)]
    /// # pub struct rpipe_endpoint_t (Endpoint);
    /// # impl PassByPointer for rpipe_endpoint_t { }
    /// /// Create a pipeline, represented as two linked endpoints.  Both
    /// /// pointers must be non-NULL and point to a valid memory location.
    /// /// Each endpoint must be freed to avoid a resource leak.
    /// #[no_mangle]
    /// pub unsafe extern "C" fn rpipe_new(
    ///     left: *mut *mut rpipe_endpoint_t,
    ///     right: *mut *mut rpipe_endpoint_t) {
    ///     let (l, r) = pipeline();
    ///     // SAFETY:
    ///     // - function docs indicate values must be freed
    ///     // - function docs indicate left and right are not NULL and valid
    ///     unsafe {
    ///         rpipe_endpoint_t(l).ptr_to_arg_out(left);
    ///         rpipe_endpoint_t(r).ptr_to_arg_out(right);
    ///     }
    /// }
    /// ```
    unsafe fn ptr_to_arg_out(self, arg_out: *mut *mut Self) {
        debug_assert!(!arg_out.is_null());
        // SAFETY: see docstring
        unsafe { *arg_out = self.return_ptr() };
    }
}
