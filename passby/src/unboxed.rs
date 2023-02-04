use crate::util::check_size_and_alignment;
use std::default::Default;
use std::marker::PhantomData;
use std::mem;

/// Unboxed is used to model values that are passed by reference, but where the memory allocation
/// is handled by C. This approach allows the C code to allocate space for the value on the stack
/// or in other structs, often avoiding unnecessary heap allocations.
///
/// The two type parameters, RType and CType, must share the same alignment, and RType must not be
/// larger than CType. Functions in this type will cause a runtime panic in debug builds if these
/// requirements are violated.
///
/// If the fields of the struct are meant to be accessible to C, RType and CType may be the same
/// type, trivially ensuring the alignment and size requirements are met.
///
/// Define your C and Rust types, then a type alias parameterizing Unboxed:
///
/// ```
/// # use ffizz_passby::Unboxed;
/// #[repr(C)]
/// struct ComplexInt {
///     re: i64,
///     im: i64,
/// }
/// type UnboxedComplexInt = Unboxed<ComplexInt, ComplexInt>;
/// ```
///
/// Then call static methods on that type alias.
///
/// # Opaque CType
///
/// It is _not_ a requirement that the fields of the types match. In fact, a common use of this
/// type is with an "opaque" C type that only contains a "reserved" field large enough to contain
/// the Rust type.  There is no constant way to determine the space required for a Rust value, but
/// it is possible to make a conservative guess, possibly leaving some unused space.  The suggested
/// C type is represented in Rust as
///
/// ```
/// # const N: usize = 2;
/// struct CType([u64; N]);
/// ```
///
/// for some N large enough to contain the Rust type on the
/// required platforms.  In C, this type would be defined as
///
/// ```text
/// struct ctype_t {
///     _reserved size_t[N];
/// }
/// ```
///
/// for the same N.  The types must also have the same alignment; typically using `size_t`
/// accomplishes this.
///
/// # Constructors
///
/// This type provides two functions useful for initialization of a CType given a value of type
/// RType: `to_out_param` takes an "output argument" pointing to an uninitialized value, and
/// initializes it; while `return_val` returns a struct value that can be used to initialize a C
/// variable.  Both function similarly, so choose the one that makes the most sense for your API.
/// For example, a constructor which can also return an error may prefer to put the error in the
/// return value and use `to_out_param`.
///
/// # Safety
///
/// C allows uninitialized values, while Rust does not.  Be careful in the documentation for the C
/// API to ensure that values are properly initialized before they are used.
#[non_exhaustive]
pub struct Unboxed<RType: Sized, CType: Sized> {
    _phantom: PhantomData<(RType, CType)>,
}

impl<RType: Sized, CType: Sized> Unboxed<RType, CType> {
    /// Take a CType and return an owned value.
    ///
    /// This approach is uncommon in C APIs. It leaves behind a value in the C allocation which
    /// could be used accidentally, resulting in a use-after-free error. Prefer [`Unboxed::take_ptr`]
    /// unless the type is Copy.
    ///
    /// # Safety
    ///
    /// * cval must be a valid CType value
    pub unsafe fn take(cval: CType) -> RType {
        // SAFETY:
        //  - cval is a valid CType (see docstring)
        unsafe { Self::from_ctype(cval) }
    }

    /// Take a pointer to a CType and return an owned value.
    ///
    /// This is intended for C API functions that take a value by reference (pointer), but still
    /// "take ownership" of the value.  It leaves behind an invalid value, where any non-padding
    /// bytes of the Rust type are zeroed.  This makes use-after-free errors in the C code more
    /// likely to crash instead of silently working.  Which is about as good as it gets in C.
    ///
    /// # Safety
    ///
    /// Do _not_ pass a pointer to a Rust value to this function:
    ///
    /// ```ignore
    /// let rust_value = RustType::take_ptr_nonnull(&mut c_value); // BAD!
    /// ```
    ///
    /// This creates undefined behavior as Rust will assume `c_value` is still initialized. Use
    /// [`Unboxed::take`] in this situation.
    ///
    /// * `cptr` must not be NULL and must point to a valid CType value (see [`Unboxed::take_ptr`] for a
    ///   version allowing NULL)
    /// * The memory pointed to by `cptr` is uninitialized when this function returns.
    pub unsafe fn take_ptr_nonnull(cptr: *mut CType) -> RType {
        check_size_and_alignment::<CType, RType>();
        if cptr.is_null() {
            panic!("NULL value not allowed");
        }

        // convert cptr to a reference to MaybeUninit<RType> (which is, for the moment,
        // actually initialized)

        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        let rref = unsafe { &mut *(cptr as *mut mem::MaybeUninit<RType>) };
        let mut owned = mem::MaybeUninit::<RType>::zeroed();
        // swap the actual value for the zeroed value
        mem::swap(rref, &mut owned);

        // SAFETY:
        //  - owned contains what cptr was pointing to, which the caller guaranteed to be valid
        unsafe { owned.assume_init() }
    }

    /// Call the contained function with a shared reference to the value.
    ///
    /// # Safety
    ///
    /// * `cptr` must not be NULL and must point to a valid CType value (see [`Unboxed::with_ref`] for a
    ///   version allowing NULL).
    /// * no other thread may mutate the value pointed to by `cptr` until the function returns.
    /// * ownership of the value remains with the caller.
    pub unsafe fn with_ref_nonnull<T, F: FnOnce(&RType) -> T>(cptr: *const CType, f: F) -> T {
        check_size_and_alignment::<CType, RType>();
        if cptr.is_null() {
            panic!("NULL value not allowed");
        }

        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        f(unsafe { &*(cptr as *const RType) })
    }

    /// Call the contained function with an exclusive reference to the data type.
    ///
    /// # Safety
    ///
    /// * `cptr` must not be NULL and must point to a valid CType value (see [`Unboxed::with_ref_mut`] for a
    ///   version allowing NULL).
    /// * No other thread may _access_ the value pointed to by `cptr` until the function returns.
    /// * Ownership of the value remains with the caller.
    pub unsafe fn with_ref_mut_nonnull<T, F: FnOnce(&mut RType) -> T>(cptr: *mut CType, f: F) -> T {
        check_size_and_alignment::<CType, RType>();
        if cptr.is_null() {
            panic!("NULL value not allowed");
        }

        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        f(unsafe { &mut *(cptr as *mut RType) })
    }

    /// Return a CType containing `rval`, moving `rval` in the process.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the value is eventually freed.
    pub unsafe fn return_val(rval: RType) -> CType {
        Self::into_ctype(rval)
    }

    /// Initialize the value pointed to arg_out with `rval`, "moving" `rval` into the pointer.
    ///
    /// If the pointer is NULL, `rval` is dropped.  Use [`Unboxed::to_out_param_nonnull`] to
    /// panic in this situation.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the value is eventually freed.
    /// * If not NULL, `arg_out` must point to valid, properly aligned memory for CType.
    pub unsafe fn to_out_param(rval: RType, arg_out: *mut CType) {
        if !arg_out.is_null() {
            // SAFETY:
            //  - arg_out is not NULL (just checked)
            //  - arg_out is properly aligned and points to valid memory (see docstring)
            unsafe { *arg_out = Self::into_ctype(rval) };
        }
    }

    /// Initialize the value pointed to arg_out with `rval`, "moving" `rval` into the pointer.
    ///
    /// If the pointer is NULL, this method will panic.
    ///
    /// # Safety
    ///
    /// * The caller must ensure that the value is eventually freed.
    /// * `arg_out` must not be NULL and must point to valid, properly aligned memory for CType.
    pub unsafe fn to_out_param_nonnull(rval: RType, arg_out: *mut CType) {
        if arg_out.is_null() {
            panic!("out param pointer is NULL");
        }
        // SAFETY:
        //  - arg_out is not NULL (see docstring)
        //  - arg_out is properly aligned and points to valid memory (see docstring)
        unsafe { *arg_out = Self::into_ctype(rval) };
    }

    /// Transmute a Rust value into a C value.
    fn into_ctype(rval: RType) -> CType {
        check_size_and_alignment::<CType, RType>();

        // This looks like a lot of code, but most of it is type arithmetic.  Only the
        // `std::ptr::copy` could potentially generate machine instructions, and in many cases even
        // that will be optimized away.

        // create a new value of type CType, uninitialized, and make a pointer to it
        let mut cval = mem::MaybeUninit::<CType>::uninit();
        let cptr = &mut cval as *mut mem::MaybeUninit<CType>;

        // create a pointer to rval
        let selfptr = (&mem::MaybeUninit::<RType>::new(rval)) as *const mem::MaybeUninit<RType>;

        // cast cptr to a pointer to RType
        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        let dest = unsafe { cptr as *mut mem::MaybeUninit<RType> };

        // copy the data
        // SAFETY:
        // - selfptr is valid for a read of 1 x RType (it's of type MaybeUninit, but was
        //   initialized)
        // - dest is valid for write of 1 x RType
        // - both are properly aligned (Rust ensures this)
        unsafe { std::ptr::copy(selfptr, dest, 1) };

        // SAFETY: dest pointed to cval, which is now valid
        unsafe { cval.assume_init() }
    }

    /// Transmute a C value into a Rust value.
    ///
    /// # Safety
    ///
    /// * `cval` must be a valid CType; that is, when interpreted as an RType (possibly with
    ///   tailing padding bytes), it must be a valid RType.
    unsafe fn from_ctype(cval: CType) -> RType {
        check_size_and_alignment::<CType, RType>();

        // wrap cval in a MaybeUninit.  It is initialized right now, but will not be
        // after the transmute_copy below.
        let cval = mem::MaybeUninit::new(cval);

        // SAFETY:
        //  - cval is a valid instance of CType, so its bytes interpreted as RType are valid
        //  (see docstring)
        //  - CType is larger than RType (guaranteed by check_size_and_alignment)
        unsafe { mem::transmute_copy(&cval) }
    }
}

impl<RType: Sized + Default, CType: Sized> Unboxed<RType, CType> {
    /// Call the contained function with a shared reference to the value.
    ///
    /// If the given pointer is NULL, the contained function is called with a reference to RType's
    /// default value, which is subsequently dropped.
    ///
    /// # Safety
    ///
    /// * If not NULL, `cptr` must point to a valid CType value.
    /// * No other thread may mutate the value pointed to by `cptr` until the function returns.
    /// * Ownership of the value remains with the caller.
    pub unsafe fn with_ref<T, F: FnOnce(&RType) -> T>(cptr: *const CType, f: F) -> T {
        check_size_and_alignment::<CType, RType>();
        if cptr.is_null() {
            let nullval = RType::default();
            return f(&nullval);
        }

        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        f(unsafe { &*(cptr as *const RType) })
    }

    /// Call the contained function with an exclusive reference to the data type.
    ///
    /// If the given pointer is NULL, the contained function is called with a reference to RType's
    /// default value, which is subsequently dropped.
    ///
    /// # Safety
    ///
    /// * If not NULL, `cptr` must point to a valid CType value.
    /// * No other thread may _access_ the value pointed to by `cptr` until the function returns.
    /// * Ownership of the value remains with the caller.
    pub unsafe fn with_ref_mut<T, F: FnOnce(&mut RType) -> T>(cptr: *mut CType, f: F) -> T {
        check_size_and_alignment::<CType, RType>();
        if cptr.is_null() {
            let mut nullval = RType::default();
            return f(&mut nullval);
        }

        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        f(unsafe { &mut *(cptr as *mut RType) })
    }

    /// Take a pointer to a CType and return an owned value.
    ///
    /// This is similar to [`Unboxed::take_ptr_nonnull`], but if given a NULL pointer will return the
    /// default value.
    ///
    /// # Safety
    ///
    /// * If not NULL, `cptr` must point to a valid CType value.
    /// * The memory pointed to by `cptr` is uninitialized when this function returns.
    pub unsafe fn take_ptr(cptr: *mut CType) -> RType {
        check_size_and_alignment::<CType, RType>();
        if cptr.is_null() {
            return RType::default();
        }

        // convert cptr to a reference to MaybeUninit<RType> (which is, for the moment,
        // actually initialized)
        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        let rref = unsafe { &mut *(cptr as *mut mem::MaybeUninit<RType>) };
        let mut owned = mem::MaybeUninit::<RType>::zeroed();

        // swap the actual value for the zeroed value
        mem::swap(rref, &mut owned);

        // SAFETY:
        //  - owned contains what cptr was pointing to, which the caller guaranteed to be valid
        unsafe { owned.assume_init() }
    }
}

#[cfg(test)]
mod test {
    mod size_panic {
        use super::super::*;
        struct TwoInts(u64, u64);
        struct OneInt(u64);

        type UnboxedTwoInts = Unboxed<TwoInts, OneInt>;

        #[test]
        #[should_panic]
        fn test() {
            let cval = OneInt(10);
            unsafe {
                UnboxedTwoInts::with_ref_nonnull(&cval as *const OneInt, |_rval| {});
            }
        }
    }

    mod align_panic {
        use super::super::*;
        struct OneInt(u64);
        struct EightBytes([u8; 8]);

        type UnboxedOneInt = Unboxed<OneInt, EightBytes>;

        #[test]
        #[should_panic]
        fn test() {
            let cval = EightBytes([0u8; 8]);
            unsafe {
                UnboxedOneInt::with_ref_nonnull(&cval as *const EightBytes, |_rval| {});
            }
        }
    }

    use super::*;
    #[derive(Default)]
    struct RType(u32, u64);
    struct CType([u64; 3]); // NOTE: larger than RType

    type UnboxedTuple = Unboxed<RType, CType>;

    #[test]
    fn intialize_and_with_methods() {
        unsafe {
            let mut cval = mem::MaybeUninit::<CType>::uninit();
            UnboxedTuple::to_out_param(RType(10, 20), cval.as_mut_ptr());
            let mut cval = cval.assume_init();

            UnboxedTuple::with_ref_nonnull(&cval, |rref| {
                assert_eq!(rref.0, 10);
                assert_eq!(rref.1, 20);
            });

            UnboxedTuple::with_ref_mut_nonnull(&mut cval, |rref| {
                assert_eq!(rref.0, 10);
                assert_eq!(rref.1, 20);
                rref.0 = 30;
            });

            UnboxedTuple::with_ref_mut(&mut cval, |rref| {
                assert_eq!(rref.0, 30);
                rref.0 += 1;
                assert_eq!(rref.1, 20);
                rref.1 += 1;
            });

            UnboxedTuple::with_ref(&cval, |rref| {
                assert_eq!(rref.0, 31);
                assert_eq!(rref.1, 21);
            });

            let rval = UnboxedTuple::take(cval);
            assert_eq!(rval.0, 31);
            assert_eq!(rval.1, 21);

            let mut cval = mem::MaybeUninit::<CType>::uninit();
            UnboxedTuple::to_out_param_nonnull(RType(100, 200), cval.as_mut_ptr());
            let cval = cval.assume_init();

            let rval = UnboxedTuple::take(cval);
            assert_eq!(rval.0, 100);
            assert_eq!(rval.1, 200);
        }
    }

    #[test]
    fn with_null_ptrs() {
        unsafe {
            UnboxedTuple::with_ref_mut(std::ptr::null_mut(), |rref| {
                assert_eq!(rref.0, 0);
                assert_eq!(rref.1, 0);
                rref.1 += 1;
            });

            UnboxedTuple::with_ref(std::ptr::null(), |rref| {
                assert_eq!(rref.0, 0);
                assert_eq!(rref.1, 0);
            });
        }
    }

    #[test]
    #[should_panic]
    fn with_ref_nonnull_null() {
        unsafe {
            UnboxedTuple::with_ref_nonnull(std::ptr::null(), |_| {});
        }
    }

    #[test]
    #[should_panic]
    fn with_ref_mut_nonnull_null() {
        unsafe {
            UnboxedTuple::with_ref_mut_nonnull(std::ptr::null_mut(), |_| {});
        }
    }

    #[test]
    fn to_out_param_null() {
        unsafe {
            UnboxedTuple::to_out_param(RType(10, 20), std::ptr::null_mut());
            // nothing happens
        }
    }

    #[test]
    #[should_panic]
    fn to_out_param_nonnull_null() {
        unsafe {
            UnboxedTuple::to_out_param_nonnull(RType(10, 20), std::ptr::null_mut());
            // nothing happens
        }
    }

    #[test]
    fn return_val() {
        unsafe {
            let cval = UnboxedTuple::return_val(RType(10, 20));
            let rval = UnboxedTuple::take(cval);
            assert_eq!(rval.0, 10);
            assert_eq!(rval.1, 20);
        }
    }

    fn take_ptr_test(nonnull: bool) {
        unsafe {
            // allocate enough bytes for a cval without initializing them
            let cval = Box::new(mem::MaybeUninit::<CType>::uninit());
            let cvalptr = Box::into_raw(cval) as *mut CType;

            // initialize the value
            UnboxedTuple::to_out_param(RType(10, 20), cvalptr);

            // take the value and leave behind zeroed memory
            let rval = if nonnull {
                UnboxedTuple::take_ptr_nonnull(cvalptr)
            } else {
                UnboxedTuple::take_ptr(cvalptr)
            };
            assert_eq!(rval.0, 10);
            assert_eq!(rval.1, 20);

            // Verify that the memory is zeroed -- don't do this IRL!  NOTE: in practice only the
            // non-padding bytes of the value are actually zeroed, so we cannot assert that all of
            // the bytes pointed to by cvalptr are zero.
            let zeroedref = unsafe { &*(cvalptr as *const RType) };
            assert_eq!(zeroedref.0, 0);
            assert_eq!(zeroedref.1, 0);

            // deallocate by turning cvalptr back into a Box and dropping the Box, but
            // using MaybeUninit to prevent dropping the (invalid) enclosed CType.
            unsafe { Box::from_raw(cvalptr as *mut mem::MaybeUninit<CType>) };
        }
    }

    #[test]
    fn take_ptr() {
        take_ptr_test(false);
    }

    #[test]
    fn take_ptr_null() {
        unsafe {
            let rval = UnboxedTuple::take_ptr(std::ptr::null_mut());
            assert_eq!(rval.0, 0);
            assert_eq!(rval.1, 0);
        }
    }

    #[test]
    fn take_ptr_nonnull() {
        take_ptr_test(true);
    }

    #[test]
    #[should_panic]
    fn take_ptr_nonnull_null() {
        unsafe {
            UnboxedTuple::take_ptr_nonnull(std::ptr::null_mut());
        }
    }
}
