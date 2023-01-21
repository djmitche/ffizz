use std::mem;

/// This trait supports structs allocated by C but managed by Rust.
///
/// This is useful for commonly-used data types that have a fixed size, to avoid allocating
/// the data type itself on the heap.
///
/// This approach uses a "regular" Rust type in the Rust code, and a C type with a `_reserved`
/// field to reserve the space on the C stack.  The tricky bit is to convince the C code to
/// allocate enough space to store the Rust value.  There is no constant way to determine the space
/// required for a Rust value, but it is possible to make a conservative guess, possibly leaving
/// some unused space.  The suggested C type is `struct CType([u64; N])` for some N large enough to
/// contain the Rust type on the required platforms.  In C, this type would be defined as `struct
/// ctype_t { _reserved uint64_t[N]; }` for the same N.  The types must also have the same alignment.
///
/// This type contains debug assertions regarding the size of the Rust and C types, and will fail
/// at runtime if the alignment or size of the two types is not as required.
///
/// This type provides two functions useful for initialization of a type: `initialize` takes an
/// "out arg" pointing to an uninitialized value, and initializes it; while `return_val` simply
/// returns a struct value that can be used to initialize a variable.  Both function similarly,
/// so choose the one that makes the most senes for your API.  For example, a constructor which
/// can also return an error may prefer to put the error in the return value and use initialize.
///
/// # Safety
///
/// C allows uninitialized values, while Rust does not.  Be careful in the documentation for the C
/// API to ensure that values are properly initialized before they are used.
pub trait OpaqueStruct: Sized {
    /// The C representation of this type.  This must have the same alignment as Self
    /// and its size must not be less than that of Self.
    type CType: Sized;

    /// Get the value of this type used to represent a NULL pointer.
    ///
    /// For types that have a natural zero value, this can provide a shortcut for a C caller:
    /// instead of initializing a struct with the zero value and passing a pointer to it, the
    /// caller can simply pass NULL.
    ///
    /// The default implementation panics.
    fn null_value() -> Self {
        panic!("NULL pointer is not allowed")
    }

    /// Call the contained function with a shared reference to the data type.
    ///
    /// # Safety
    /// * for types defining [`null_value`]: cptr must be NULL or point to a valid CType value
    /// * for types not defining [`null_value`]: cptr must not be NULL and must point to a valid
    ///   CType value
    /// * no other thread may mutate the value pointed to by CType until with_ref returns.
    unsafe fn with_ref<T, F: Fn(&Self) -> T>(cptr: *const Self::CType, f: F) -> T {
        check_size_and_alignment::<Self::CType, Self>();
        if cptr.is_null() {
            return f(&Self::null_value());
        }

        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        f(unsafe { &*(cptr as *const Self) })
    }

    /// Call the contained function with an exclusive reference to the data type.
    ///
    /// # Safety
    ///
    /// * for types defining [`null_value`]: cptr must be NULL or point to a valid CType value
    /// * for types not defining [`null_value`]: cptr must not be NULL and must point to a valid
    ///   CType value
    /// * no other thread may access the value pointed to by CType until with_ref_mut returns.
    unsafe fn with_ref_mut<T, F: Fn(&mut Self) -> T>(cptr: *mut Self::CType, f: F) -> T {
        check_size_and_alignment::<Self::CType, Self>();
        if cptr.is_null() {
            let mut null = Self::null_value();
            return f(&mut null);
        }

        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        f(unsafe { &mut *(cptr as *mut Self) })
    }

    /// Initialize the value pointed to cptr with rval, "moving" rval into the pointer.
    ///
    /// # Safety
    ///
    /// * cptr must not be NULL, must be aligned for CType, and must have enough space for CType.
    /// * to avoid a leak, the value must eventually be moved out of *cptr and into a Rust value
    ///   to be dropped (see [`OpaqueStruct::take`])
    unsafe fn initialize(cptr: *mut Self::CType, rval: Self) {
        check_size_and_alignment::<Self::CType, Self>();
        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        // - MaybeUninit<Self> has the same layout as Self
        let rref = unsafe { &mut *(cptr as *mut mem::MaybeUninit<Self>) };
        rref.write(rval);
    }

    /// Return a CType containing self, moving self in the process.
    ///
    /// # Safety
    ///
    /// * to avoid a leak, the value must eventually be moved out of the return value
    ///   and into a Rust value to be dropped (see [`OpaqueStruct::take`])
    unsafe fn return_val(self) -> Self::CType {
        check_size_and_alignment::<Self::CType, Self>();
        // create a new value of type Self::CType, uninitialized, and make a pointer to it
        let mut cval = mem::MaybeUninit::<Self::CType>::uninit();
        let cptr = &mut cval as *mut mem::MaybeUninit<Self::CType>;

        // create a pointer to self
        let selfptr = (&mem::MaybeUninit::<Self>::new(self)) as *const mem::MaybeUninit<Self>;

        // cast cptr to a pointer to Self
        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        let dest = unsafe { cptr as *mut mem::MaybeUninit<Self> };

        // copy the data
        // SAFETY:
        // - selfptr is valid for a read of 1 x Self (it's of type MaybeUninit, but was
        //   initialized)
        // - dest is valid for write of 1 x Self
        // - both are properly aligned (Rust ensures this)
        unsafe { std::ptr::copy(selfptr, dest, 1) };

        // SAFETY: dest pointed to cval, which is now valid
        unsafe { cval.assume_init() }
    }

    /// Take a CType and return an owned value.
    ///
    /// This method is used for "xxx_free" functions, which simply drop the resulting owned value.
    ///
    /// It is also used for functions which are documented in the C API as consuming the given value,
    /// saving the caller the trouble of separately freeing the value.
    ///
    /// The memory pointed to by cptr is zeroed to prevent accidental re-use.
    ///
    /// # Safety
    ///
    /// * for types defining [`null_value`]: cptr must be NULL or point to a valid CType value
    /// * for types not defining [`null_value`]: cptr must not be NULL and must point to a valid
    ///   CType value
    /// * the memory pointed to by cptr is uninitialized when this function returns
    unsafe fn take(cptr: *mut Self::CType) -> Self {
        check_size_and_alignment::<Self::CType, Self>();
        if cptr.is_null() {
            return Self::null_value();
        }

        // convert cptr to a reference to Self
        // SAFETY:
        // - casting to a pointer type with the same alignment and smaller size
        let rref = unsafe { &mut *(cptr as *mut mem::MaybeUninit<Self>) };

        // replace that with a zero value, and get what was there as an owned
        // value
        let owned = mem::replace(rref, mem::MaybeUninit::<Self>::zeroed());

        // SAFETY:
        //  - owned contains what cptr was pointing to, which the caller guaranteed to be valid
        unsafe { owned.assume_init() }
    }
}

/// Verify the size and alignment requirements are met.  These will compile to nothing if the
/// requirements are met, and will compile to `debug_assert!(false)` if they are not met, causing
/// all trait methods to panic.  That should be enough to get someone's attention!
fn check_size_and_alignment<CType: Sized, RType: Sized>() {
    debug_assert!(mem::size_of::<RType>() <= mem::size_of::<CType>());
    debug_assert!(mem::align_of::<RType>() == mem::align_of::<CType>());
}

mod test {
    mod size_panic {
        use crate::opaque::*;
        struct TwoInts(u64, u64);
        struct OneInt(u64);

        impl OpaqueStruct for TwoInts {
            type CType = OneInt; // uhoh! smaller than TwoInts!
        }

        #[test]
        #[should_panic]
        fn test() {
            let cval = OneInt(10);
            unsafe {
                TwoInts::with_ref(&cval as *const OneInt, |_rval| {});
            }
        }
    }

    mod align_panic {
        use crate::opaque::*;
        struct OneInt(u64);
        struct EightBytes([u8; 8]);

        impl OpaqueStruct for OneInt {
            type CType = EightBytes; // uhoh! different alignment than OneInt!
        }

        #[test]
        #[should_panic]
        fn test() {
            let cval = EightBytes([0u8; 8]);
            unsafe {
                OneInt::with_ref(&cval as *const EightBytes, |_rval| {});
            }
        }
    }

    mod init_and_use {
        use crate::opaque::*;
        struct RType(u32, u64);
        struct CType([u64; 2]);

        impl OpaqueStruct for RType {
            type CType = CType;
        }

        #[test]
        fn test() {
            unsafe {
                let mut cval = mem::MaybeUninit::<CType>::uninit();
                RType::initialize(cval.as_mut_ptr(), RType(10, 10));

                RType::with_ref(cval.as_ptr(), |rref| {
                    assert_eq!(rref.0, 10);
                });

                RType::with_ref_mut(cval.as_mut_ptr(), |rref| {
                    assert_eq!(rref.0, 10);
                    rref.0 = 20;
                });

                RType::with_ref(cval.as_ptr(), |rref| {
                    assert_eq!(rref.0, 20);
                });

                RType::take(cval.as_mut_ptr()); // ..and implicitly drop
            }
        }
    }
}
