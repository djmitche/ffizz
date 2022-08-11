#[doc = include_str!("pass-by-value-doc.md")]
pub trait PassByValue: Sized {
    /// The Rust representation of this type.
    type RustType;

    /// Convert a C value to a Rust value.
    ///
    /// # Safety
    ///
    /// The implementation of this method assumes that `self` is a valid instance of Self.
    #[allow(clippy::wrong_self_convention)]
    unsafe fn from_ctype(self) -> Self::RustType;

    /// Convert a Rust value to a C value.
    fn as_ctype(arg: Self::RustType) -> Self;

    /// Take a value from C as an argument.
    ///
    /// # Safety
    ///
    /// - `self` must be a valid instance of the C type.  This is typically ensured either by
    ///   requiring that C code not modify it, or by defining the valid values in C comments.
    ///
    /// # Example
    ///
    /// ```
    /// # use uuid::Uuid;
    /// # use ffizz_passby::PassByValue;
    /// # #[allow(non_camel_case_types)]
    /// # pub struct foo_uuid_t([u8; 16]);
    /// # impl PassByValue for foo_uuid_t {
    /// #     type RustType = Uuid;
    /// #     unsafe fn from_ctype(self) -> Self::RustType { todo!() }
    /// #     fn as_ctype(arg: Uuid) -> Self { todo!() }
    /// # }
    /// /// Determine the version for the given UUID.  The given UUID must be valid.
    /// #[no_mangle]
    /// pub unsafe extern "C" fn uuid_version(uuid: foo_uuid_t) -> usize {
    ///     // SAFETY:
    ///     // - uuid is a valid foo_uuid_t (see docstring)
    ///     // - uuid is Copy so ownership doesn't matter
    ///     let uuid = unsafe { foo_uuid_t::val_from_arg(uuid) };
    ///     return uuid.get_version_num()
    /// }
    /// ```
    unsafe fn val_from_arg(arg: Self) -> Self::RustType {
        // SAFETY:
        //  - arg is a valid CType (see docstring)
        unsafe { arg.from_ctype() }
    }

    /// Take a value from C as a pointer argument, replacing it with the given value.  This is used
    /// to invalidate the C value as an additional assurance against subsequent use of the value.
    ///
    /// Most uses of this trait do not require invalidation to ensure correctness, so it is unusual
    /// to use this method.
    ///
    /// # Safety
    ///
    /// - arg must not be NULL
    /// - *arg must be a valid, properly aligned instance of the C type
    ///
    /// # Example
    ///
    /// Consider a `foo_file_t` that wraps a file descriptor.  The C API can avoid use of this
    /// descriptor after close by invalidating the file descriptor when closing.
    ///
    /// Note that the Rust type used here must _not_ automatically close the file on drop!
    ///
    /// ```rust
    /// # use ffizz_passby::PassByValue;
    /// #[repr(C)]
    /// struct foo_file_t { fd: i64 }
    /// # struct File(i64);
    /// # impl File {
    /// #     fn close(self) { todo!() }
    /// # }
    /// # impl PassByValue for foo_file_t {
    /// #     type RustType = File;
    /// #     unsafe fn from_ctype(self) -> Self::RustType { todo!() }
    /// #     fn as_ctype(arg: Self::RustType) -> Self { todo!() }
    /// # }
    /// /// Close a foo_file_t. The given file must not be NULL and must point to a valid, open
    /// /// foo_file_t. The file cannot be used after this call.
    /// #[no_mangle]
    /// pub unsafe extern "C" fn foo_file_close(file: *mut foo_file_t) {
    ///     // SAFETY:
    ///     // - file is not NULL (see docstring)
    ///     // - *file is a valid foo_file_t (see docstring)
    ///     let file = unsafe {
    ///         foo_file_t::take_val_from_arg(file, foo_file_t { fd: -1 })
    ///     };
    ///     file.close();
    /// }
    unsafe fn take_val_from_arg(arg: *mut Self, mut replacement: Self) -> Self::RustType {
        // SAFETY:
        //  - arg is valid (see docstring)
        //  - replacement is valid and aligned (guaranteed by Rust)
        unsafe { std::ptr::swap(arg, &mut replacement) };
        // SAFETY:
        //  - replacement (formerly *arg) is a valid CType (see docstring)
        unsafe { PassByValue::val_from_arg(replacement) }
    }

    /// Return a value to C
    ///
    /// # Safety
    ///
    /// - if the value is allocated, the caller must ensure that the value is eventually freed
    ///
    /// # Example
    ///
    /// ```rust
    /// # use uuid::Uuid;
    /// # use ffizz_passby::PassByValue;
    /// # #[allow(non_camel_case_types)]
    /// # pub struct foo_uuid_t([u8; 16]);
    /// # impl PassByValue for foo_uuid_t {
    /// #     type RustType = Uuid;
    /// #     unsafe fn from_ctype(self) -> Self::RustType { todo!() }
    /// #     fn as_ctype(arg: Uuid) -> Self { todo!() }
    /// # }
    /// /// Create a new, randomly-generated UUID.
    /// #[no_mangle]
    /// pub unsafe extern "C" fn make_uuid() -> foo_uuid_t {
    ///    // SAFETY:
    ///    // - value is not allocated; no concerns
    ///    unsafe { foo_uuid_t::return_val(Uuid::new_v4()) }
    /// }
    /// ```
    unsafe fn return_val(arg: Self::RustType) -> Self {
        Self::as_ctype(arg)
    }

    /// Return a value to C, via an "output parameter"
    ///
    /// # Safety
    ///
    /// - `arg_out` must not be NULL and must be properly aligned and pointing to valid memory
    ///   of the size of CType.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use uuid::Uuid;
    /// # use ffizz_passby::PassByValue;
    /// # #[allow(non_camel_case_types)]
    /// # pub struct foo_uuid_t([u8; 16]);
    /// # impl PassByValue for foo_uuid_t {
    /// #     type RustType = Uuid;
    /// #     unsafe fn from_ctype(self) -> Self::RustType { todo!() }
    /// #     fn as_ctype(arg: Uuid) -> Self { todo!() }
    /// # }
    /// /// Create a pair of UUIDs entangled at the quantum level.  Both pointers
    /// /// must be properly aligned and pointing to valid memory to contain a
    /// /// foo_uuid_t.
    /// #[no_mangle]
    /// pub unsafe extern "C" fn foo_uuid_entangled_pair(
    ///     u1: *mut foo_uuid_t, u2: *mut foo_uuid_t) {
    ///    // SAFETY:
    ///    // - u1, u2 are not NULL, properly aligned, and point to valid memory
    ///    //   (see docstring)
    ///    unsafe {
    ///        // MVP: just use random uuids until quantum entanglement is possible
    ///        foo_uuid_t::val_to_arg_out(Uuid::new_v4(), u1);
    ///        foo_uuid_t::val_to_arg_out(Uuid::new_v4(), u2);
    ///    }
    /// }
    /// ```
    unsafe fn val_to_arg_out(val: Self::RustType, arg_out: *mut Self) {
        debug_assert!(!arg_out.is_null());
        // SAFETY:
        //  - arg_out is not NULL (see docstring)
        //  - arg_out is properly aligned and points to valid memory (see docstring)
        unsafe { *arg_out = Self::as_ctype(val) };
    }
}
