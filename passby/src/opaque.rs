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
/// ctype_t { _reserved uint64_t[N] }` for the same N.  The types must also have the same alignment.
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

    /// TODO: doc
    unsafe fn with_ref<T, F: Fn(&Self) -> T>(cptr: *const Self::CType, f: F) -> T {
        let rref: &Self = unsafe { transmute_ref(cptr) };
        f(rref)
    }

    // can't use this with uninitialized!!
    /// TODO: doc
    unsafe fn with_mut_ref<T, F: Fn(&mut Self) -> T>(cptr: *mut Self::CType, f: F) -> T {
        let rref: &mut Self = unsafe { transmute_mut_ref(cptr) };
        f(rref)
    }

    /// TODO: doc
    unsafe fn initialize(ptr: *mut Self::CType, rval: Self) {
        let rref: &mut mem::MaybeUninit<Self> = unsafe { transmute_mut_ref(ptr) };
        rref.write(rval);
    }

    /// TODO: doc
    unsafe fn return_val(self) -> Self::CType {
        check_size_and_alignment::<Self::CType, Self>();
        unsafe { mem::transmute_copy(&mem::ManuallyDrop::new(self)) }
    }

    /// TODO: doc
    unsafe fn take(cptr: *mut Self::CType) -> Self {
        let mut tmp = mem::MaybeUninit::<Self>::zeroed();
        let rref: &mut mem::MaybeUninit<Self> = unsafe { transmute_mut_ref(cptr) };
        mem::swap(&mut tmp, rref);
        unsafe { tmp.assume_init() }
    }
}

/// Verify the size and alignment requirements are met.  These will compile to nothing if the
/// requirements are met, and will compile to `debug_assert!(false)` if they are not met, causing
/// all trait methods to panic.  That should be enough to get someone's attention!
fn check_size_and_alignment<CType: Sized, RType: Sized>() {
    debug_assert!(mem::size_of::<RType>() <= mem::size_of::<CType>());
    debug_assert!(mem::align_of::<RType>() == mem::align_of::<CType>());
}

/// TODO: doc, safety
unsafe fn transmute_ref<'a, CType: Sized, RType: Sized>(cval: *const CType) -> &'a RType {
    check_size_and_alignment::<CType, RType>();
    let cref = unsafe { &*cval };
    unsafe { std::mem::transmute(cref) }
}

/// TODO: doc, safety
unsafe fn transmute_mut_ref<'a, CType: Sized, RType: Sized>(cval: *mut CType) -> &'a mut RType {
    check_size_and_alignment::<CType, RType>();
    let cref = unsafe { &mut *cval };
    unsafe { std::mem::transmute(cref) }
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

                RType::with_mut_ref(cval.as_mut_ptr(), |rref| {
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
