use std::default::Default;
use std::marker::PhantomData;

/// Boxed is used to model values that are passed by reference and where their memory allocation is
/// managed entirely by Rust.  These are represented in the C API by a pointer, with "new" and
/// "free" functions handling creation and destruction.
///
/// The value may be opaque to C, so that it may not access fields in the struct directly, in which
/// case `RType` can be any Rust type.  Otherwise, if a C structure is provided, you must use
/// `#[repr(C)]` to ensure that C and Rust lay out the struct identically.
///
/// # Example
///
/// Define your C and Rust types, then a type alias parameterizing Boxed:
///
/// ```
/// # use ffizz_passby::Boxed;
/// struct System {
///     // ...
/// }
/// type BoxedSystem = Boxed<System>;
/// ```
///
/// Then call static methods on that type alias.
#[non_exhaustive]
pub struct Boxed<RType: Sized> {
    _phantom: PhantomData<RType>,
}

impl<RType: Sized> Boxed<RType> {
    /// Take a value from C as an argument, taking ownership of the value it points to.
    ///
    /// Be careful that the C API documents that the passed pointer cannot be used after this
    /// function is called.
    ///
    /// If you would like to borrow the value, but leave ownership with the calling C code, use
    /// [`Boxed::with_ref`] or its variants.
    ///
    /// This function is most common in "free" functions, but can also be used in contexts where it
    /// is ergonomic for the called function to consume the value.  For example, a database
    /// connections's `execute` method might reasonably consume a query argument.
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
    /// * `arg` must not be NULL (see [`Boxed::take`] for a version allowing NULL).
    /// * `arg` must be a value returned from `Box::into_raw` (via [`Boxed::return_val`] or [`Boxed::to_out_param`] or a variant).
    /// * `arg` becomes invalid and must not be used after this call.
    pub unsafe fn take_nonnull(arg: *mut RType) -> RType {
        debug_assert!(!arg.is_null());
        // SAFETY: see docstring
        unsafe { *(Box::from_raw(arg)) }
    }

    /// Call the contained function with a shared reference to the value.
    ///
    /// # Safety
    ///
    /// * `arg` must not be NULL (see [`Boxed::with_ref`] for a version allowing NULL).
    /// * No other thread may mutate the value pointed to by `arg` until this function returns.
    /// * Ownership of the value remains with the caller.
    pub unsafe fn with_ref_nonnull<T, F: FnOnce(&RType) -> T>(arg: *const RType, f: F) -> T {
        if arg.is_null() {
            panic!("NULL value not allowed");
        }
        // SAFETY:
        // - pointer came from Box::into_raw, so has proper size and alignment
        f(unsafe { &*(arg as *const RType) })
    }

    /// Call the contained function with an exclusive reference to the value.
    ///
    /// # Safety
    ///
    /// * `arg` must not be NULL (see [`Boxed::with_ref_mut`] for a version allowing null)
    /// * No other thread may _access_ the value pointed to by `arg` until this function returns.
    /// * Ownership of the value remains with the caller.
    pub unsafe fn with_ref_mut_nonnull<T, F: FnOnce(&mut RType) -> T>(arg: *mut RType, f: F) -> T {
        if arg.is_null() {
            panic!("NULL value not allowed");
        }
        // SAFETY:
        // - pointer came from Box::into_raw, so has proper size and alignment
        f(unsafe { &mut *arg })
    }

    /// Return a value to C, boxing the value and transferring ownership.
    ///
    /// This method is most often used in constructors, to return the built value.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the value is eventually freed.
    pub unsafe fn return_val(rval: RType) -> *mut RType {
        // SAFETY: return_val_boxed and return_val have the same safety requirements.
        unsafe { Self::return_val_boxed(Box::new(rval)) }
    }

    /// Return a boxed value to C, transferring ownership.
    ///
    /// This is an alternative to [`Boxed::return_val`] for use when the value is already boxed.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the value is eventually freed.
    pub unsafe fn return_val_boxed(rval: Box<RType>) -> *mut RType {
        Box::into_raw(rval)
    }

    /// Return a value to C, transferring ownership, via an "output parameter".
    ///
    /// If the pointer is NULL, the value is dropped.  Use [`Boxed::to_out_param_nonnull`] to panic
    /// in this situation.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the value is eventually freed.
    /// * If not NULL, `arg_out` must point to valid, properly aligned memory for a pointer value.
    pub unsafe fn to_out_param(rval: RType, arg_out: *mut *mut RType) {
        if !arg_out.is_null() {
            // SAFETY: see docstring
            unsafe { *arg_out = Self::return_val(rval) };
        }
    }

    /// Return a value to C, transferring ownership, via an "output parameter".
    ///
    /// If the pointer is NULL, this function will panic.  Use [`Boxed::to_out_param`] to
    /// drop the value in this situation.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the value is eventually freed.
    /// * `arg_out` must not be NULL.
    /// * `arg_out` must point to valid, properly aligned memory for a pointer value.
    pub unsafe fn to_out_param_nonnull(rval: RType, arg_out: *mut *mut RType) {
        if arg_out.is_null() {
            panic!("out param pointer is NULL");
        }
        // SAFETY: see docstring
        unsafe { *arg_out = Self::return_val(rval) };
    }
}

impl<RType: Sized + Default> Boxed<RType> {
    /// Take a value from C as an argument.
    ///
    /// This function is similar to [`Boxed::take_nonnull`], but returns the default value of RType when
    /// given NULL.
    ///
    /// # Safety
    ///
    /// * `arg` must be a value returned from `Box::into_raw` (via [`Boxed::return_val`] or [`Boxed::to_out_param`] or a variant).
    /// * `arg` becomes invalid and must not be used after this call.
    pub unsafe fn take(arg: *mut RType) -> RType {
        debug_assert!(!arg.is_null());
        // SAFETY: see docstring
        unsafe { *(Box::from_raw(arg)) }
    }

    /// Call the contained function with a shared reference to the value.
    ///
    /// If the given pointer is NULL, the contained function is called with a reference to RType's
    /// default value, which is subsequently dropped.
    ///
    /// # Safety
    ///
    /// * No other thread may mutate the value pointed to by `arg` until this function returns.
    /// * Ownership of the value remains with the caller.
    pub unsafe fn with_ref<T, F: FnOnce(&RType) -> T>(arg: *const RType, f: F) -> T {
        if arg.is_null() {
            let nullval = RType::default();
            return f(&nullval);
        }

        // SAFETY:
        // - pointer is not NULL (just checked)
        // - pointer came from Box::into_raw, so has proper size and alignment
        f(unsafe { &*(arg as *const RType) })
    }

    /// Call the contained function with an exclusive reference to the value.
    ///
    /// If the given pointer is NULL, the contained function is called with a reference to RType's
    /// default value, which is subsequently dropped.
    ///
    /// # Safety
    ///
    /// * No other thread may _access_ the value pointed to by `arg` until this function returns.
    /// * Ownership of the value remains with the caller.
    pub unsafe fn with_ref_mut<T, F: FnOnce(&mut RType) -> T>(arg: *mut RType, f: F) -> T {
        if arg.is_null() {
            let mut nullval = RType::default();
            return f(&mut nullval);
        }

        // SAFETY:
        // - pointer is not NULL (just checked)
        // - pointer came from Box::into_raw, so has proper size and alignment
        f(unsafe { &mut *arg })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::mem;

    #[derive(Default)]
    struct RType(u32, u64);

    type BoxedTuple = Boxed<RType>;

    #[test]
    fn intialize_and_with_methods() {
        unsafe {
            let mut cptr = mem::MaybeUninit::<*mut RType>::uninit();
            BoxedTuple::to_out_param(RType(10, 20), cptr.as_mut_ptr());
            let cptr = cptr.assume_init();

            BoxedTuple::with_ref_nonnull(cptr, |rref| {
                assert_eq!(rref.0, 10);
                assert_eq!(rref.1, 20);
            });

            BoxedTuple::with_ref_mut_nonnull(cptr, |rref| {
                assert_eq!(rref.0, 10);
                assert_eq!(rref.1, 20);
                rref.0 = 30;
            });

            BoxedTuple::with_ref_mut(cptr, |rref| {
                assert_eq!(rref.0, 30);
                rref.0 += 1;
                assert_eq!(rref.1, 20);
                rref.1 += 1;
            });

            BoxedTuple::with_ref(cptr, |rref| {
                assert_eq!(rref.0, 31);
                assert_eq!(rref.1, 21);
            });

            let rval = BoxedTuple::take(cptr);
            assert_eq!(rval.0, 31);
            assert_eq!(rval.1, 21);

            let mut cptr = mem::MaybeUninit::<*mut RType>::uninit();
            BoxedTuple::to_out_param_nonnull(RType(100, 200), cptr.as_mut_ptr());
            let cptr = cptr.assume_init();

            let rval = BoxedTuple::take(cptr);
            assert_eq!(rval.0, 100);
            assert_eq!(rval.1, 200);
        }
    }

    #[test]
    fn with_null_ptrs() {
        unsafe {
            BoxedTuple::with_ref_mut(std::ptr::null_mut(), |rref| {
                assert_eq!(rref.0, 0);
                assert_eq!(rref.1, 0);
                rref.1 += 1;
            });

            BoxedTuple::with_ref(std::ptr::null(), |rref| {
                assert_eq!(rref.0, 0);
                assert_eq!(rref.1, 0);
            });
        }
    }

    #[test]
    #[should_panic]
    fn with_ref_nonnull_null() {
        unsafe {
            BoxedTuple::with_ref_nonnull(std::ptr::null(), |_| {});
        }
    }

    #[test]
    #[should_panic]
    fn with_ref_mut_nonnull_null() {
        unsafe {
            BoxedTuple::with_ref_mut_nonnull(std::ptr::null_mut(), |_| {});
        }
    }

    #[test]
    fn to_out_param_null() {
        unsafe {
            BoxedTuple::to_out_param(RType(10, 20), std::ptr::null_mut());
            // nothing happens
        }
    }

    #[test]
    #[should_panic]
    fn to_out_param_nonnull_null() {
        unsafe {
            BoxedTuple::to_out_param_nonnull(RType(10, 20), std::ptr::null_mut());
            // nothing happens
        }
    }

    #[test]
    fn return_val_take() {
        unsafe {
            let cptr = BoxedTuple::return_val(RType(10, 20));
            let rval = BoxedTuple::take(cptr);
            assert_eq!(rval.0, 10);
            assert_eq!(rval.1, 20);
        }
    }

    #[test]
    fn return_val_boxed_take_nonnull() {
        unsafe {
            let cptr = BoxedTuple::return_val_boxed(Box::new(RType(10, 20)));
            let rval = BoxedTuple::take_nonnull(cptr);
            assert_eq!(rval.0, 10);
            assert_eq!(rval.1, 20);
        }
    }

    #[test]
    #[should_panic]
    fn take_nnull() {
        unsafe {
            let rval = BoxedTuple::take(std::ptr::null_mut());
            assert_eq!(rval.0, 0);
            assert_eq!(rval.1, 0);
        }
    }

    #[test]
    #[should_panic]
    fn take_nonnull_null() {
        unsafe {
            BoxedTuple::take_nonnull(std::ptr::null_mut());
        }
    }
}
