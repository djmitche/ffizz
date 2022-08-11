#[doc = include_str!("pass-by-pointer-doc.md")]
pub trait PassByPointer: Sized {
    /// Take a value from C as an argument.
    ///
    /// This function is typically used to handle arguments passed from C, but because it takes
    /// ownership of the passed value, while leaving the C code with a pointer, it can lead to
    /// use-after-free errors if not used carefully.  It is most useful in situations where the
    /// C code has built up a value and it is clear from context that the Rust code takes
    /// ownership.  For example:
    ///
    /// ```c
    /// foo_query_t q = foo_query_new();
    /// foo_query_set_filter(q, "x = 10");
    /// foo_query_add_column(q, "y");
    /// foo_result_t res = foo_db_execute(db, q);
    /// ```
    ///
    /// Here it's natural to assume (but should also be documented) that the `foo_db_execute`
    /// function takes ownership of the query.
    ///
    /// # Safety
    ///
    /// - arg must not be NULL
    /// - arg must be a value returned from Box::into_raw (via return_ptr or ptr_to_arg_out)
    /// - arg becomes invalid and must not be used after this call
    ///
    /// # Example
    ///
    /// ```
    /// # #![allow(non_camel_case_types)]
    /// # use ffizz_passby::PassByPointer;
    /// # struct DBEngine { }
    /// # impl DBEngine {
    /// #     fn execute(&mut self, query: DBQuery) -> DBResult { todo!() }
    /// # }
    /// # struct DBQuery { }
    /// # struct DBResult { }
    /// # pub struct foo_db_t (DBEngine);
    /// # pub struct foo_query_t (DBQuery);
    /// # pub struct foo_result_t (DBResult);
    /// # impl PassByPointer for foo_db_t { }
    /// # impl PassByPointer for foo_query_t { }
    /// # impl PassByPointer for foo_result_t { }
    /// /// Execute a query and return the result.  The `db` and `query` arguments must be valid
    /// /// values returned from `foo_db_new` and `foo_query_new`, respectively.  This function
    /// /// consumes the query, and it must not be used after this function returns.  The result
    /// /// must be freed.
    /// #[no_mangle]
    /// pub unsafe extern "C" fn foo_query_execute(db: *mut foo_db_t, query: *mut foo_query_t) -> *mut foo_result_t {
    ///     # let mut db = foo_db_t::from_ptr_arg_ref_mut(db);
    ///     // ...
    ///     // SAFETY:
    ///     // - query is not null and is valid (see docstring)
    ///     // - query will not be used after return (see docstring)
    ///     let query = foo_query_t::take_from_ptr_arg(query);
    ///     let result = db.0.execute(query.0);
    ///     // ...
    ///     # unsafe { foo_result_t(result).return_ptr() }
    /// }
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// # #![allow(non_camel_case_types)]
    /// # use ffizz_passby::PassByPointer;
    /// # struct DBResult { }
    /// # impl DBResult {
    /// #     fn num_rows(&self) -> usize { todo!() }
    /// # }
    /// # pub struct foo_result_t (DBResult);
    /// # impl PassByPointer for foo_result_t { }
    /// /// Return the number of rows in this result.  The argument must be a valid result object.
    /// #[no_mangle]
    /// pub unsafe extern "C" fn foo_result_num_rows(result: *const foo_result_t) -> usize {
    ///     // SAFETY:
    ///     // - result is not null and is valid (see docstring)
    ///     // - result will remain valid for life of this function call (library docs state it is not threadsafe)
    ///     // - result will not be accessed by anything else concurrently (not threadsafe)
    ///     let result = unsafe { foo_result_t::from_ptr_arg_ref(result) };
    ///     result.0.num_rows()
    /// }
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// # #![allow(non_camel_case_types)]
    /// # use ffizz_passby::PassByPointer;
    /// # struct DBEngine { }
    /// # impl DBEngine {
    /// #     fn execute(&mut self, query: DBQuery) -> DBResult { todo!() }
    /// # }
    /// # struct DBQuery { }
    /// # struct DBResult { }
    /// # pub struct foo_db_t (DBEngine);
    /// # pub struct foo_query_t (DBQuery);
    /// # pub struct foo_result_t (DBResult);
    /// # impl PassByPointer for foo_db_t { }
    /// # impl PassByPointer for foo_query_t { }
    /// # impl PassByPointer for foo_result_t { }
    /// /// Execute a query and return the result.  The `db` and `query` arguments must be valid
    /// /// values returned from `foo_db_new` and `foo_query_new`, respectively.  This function
    /// /// consumes the query, and it must not be used after this function returns.  The result
    /// /// must be freed.
    /// #[no_mangle]
    /// pub unsafe extern "C" fn foo_query_execute(db: *mut foo_db_t, query: *mut foo_query_t) -> *mut foo_result_t {
    ///     // SAFETY:
    ///     // - db is not null and is valid (see docstring)
    ///     // - db will remain valid for life of this function call (library docs state it is not threadsafe)
    ///     // - db will not be accessed by anything else concurrently (not threadsafe)
    ///     let mut db = foo_db_t::from_ptr_arg_ref_mut(db);
    ///     // ...
    ///     // ...
    ///     # let query = foo_query_t::take_from_ptr_arg(query);
    ///     # let result = db.0.execute(query.0);
    ///     # unsafe { foo_result_t(result).return_ptr() }
    /// }
    /// ```
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
    ///
    /// # Example
    ///
    /// This method is most often used in constructors:
    ///
    /// ```
    /// # use ffizz_passby::PassByPointer;
    /// # struct DBEngine { }
    /// # impl DBEngine {
    /// #     fn new() -> Self { todo!() }
    /// # }
    /// # #[allow(non_camel_case_types)]
    /// # pub struct foo_db_t (DBEngine);
    /// # impl PassByPointer for foo_db_t { }
    /// /// Open a new fooDB.  The resulting foo_db_t must be freed when it is no
    /// /// longer needed.
    /// #[no_mangle]
    /// pub unsafe extern "C" fn foo_db_new() -> *mut foo_db_t {
    ///     let db = foo_db_t(DBEngine::new());
    ///     // SAFETY: function docs indicate value must be freed
    ///     unsafe { db.return_ptr() }
    /// }
    /// ```
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
