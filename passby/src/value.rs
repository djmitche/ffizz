use std::marker::PhantomData;

/// Value is used to "pass by value' semantics.
///
/// This is typically used for Copy types, such as integers or enums. For types that are not Copy,
/// [`crate::Unboxed`] is a better choice.
///
/// The two type parameters must be convertible using `Into<RType> for CType` and `From<RType> for
/// CType`. This choice of traits was made deliberately, on the assumption that `CType` is defined
/// locally to your crate, while `RType` may be a type from another crate.
///
/// # Example
///
/// Define your C and Rust types, then a type alias parameterizing Value:
///
/// ```
/// # type Uuid = i128;
/// # use ffizz_passby::Value;
/// #[repr(C)]
/// pub struct uuid_t([u8; 16]);
///
/// type UuidValue = Value<Uuid, uuid_t>;
/// ```
///
/// Then call static mtehods on that type alias.
#[non_exhaustive]
pub struct Value<RType, CType>
where
    RType: Sized,
    CType: Sized + From<RType> + Into<RType>,
{
    _phantom: PhantomData<(RType, CType)>,
}

impl<RType, CType> Value<RType, CType>
where
    // In typical usage, RType might be a type that is external to the user's crate,
    // so we cannot require any custom traits on that type.
    RType: Sized,
    CType: Sized + From<RType> + Into<RType>,
{
    /// Take a CType and return an owned value.
    ///
    /// The caller retains a copy of the value.
    pub fn take(cval: CType) -> RType {
        cval.into()
    }

    /// Return a CType containing rval, moving rval in the process.
    pub fn return_val(rval: RType) -> CType {
        CType::from(rval)
    }

    /// Initialize the value pointed to `arg_out` with rval, "moving" rval into the pointer.
    ///
    /// If the pointer is NULL, rval is dropped.  Use [`Value::to_out_param_nonnull`] to
    /// panic in this situation.
    ///
    /// # Safety
    ///
    /// * if `arg_out` is not NULL, then it must be aligned for and have enough space for
    ///   CType.
    pub unsafe fn to_out_param(rval: RType, arg_out: *mut CType) {
        if !arg_out.is_null() {
            // SAFETY:
            //  - arg_out is not NULL (just checked)
            //  - arg_out is properly aligned and points to valid memory (see docstring)
            unsafe { *arg_out = CType::from(rval) };
        }
    }

    /// Initialize the value pointed to `arg_out` with rval, "moving" rval into the pointer.
    ///
    /// If the pointer is NULL, this method will panic.
    ///
    /// # Safety
    ///
    /// * `arg_out` must not be NULL, must be aligned for CType and have enough space for CType.
    pub unsafe fn to_out_param_nonnull(rval: RType, arg_out: *mut CType) {
        if arg_out.is_null() {
            panic!("out param pointer is NULL");
        }
        // SAFETY:
        //  - arg_out is not NULL (see docstring)
        //  - arg_out is properly aligned and points to valid memory (see docstring)
        unsafe { *arg_out = CType::from(rval) };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::mem;

    #[allow(non_camel_case_types)]
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct result_t {
        is_ok: bool,
        error_code: u32,
    }

    impl Into<Result<(), u32>> for result_t {
        fn into(self) -> Result<(), u32> {
            if self.is_ok {
                Ok(())
            } else {
                Err(self.error_code)
            }
        }
    }

    impl From<Result<(), u32>> for result_t {
        fn from(res: Result<(), u32>) -> result_t {
            match res {
                Ok(_) => result_t {
                    is_ok: true,
                    error_code: 0,
                },
                Err(error_code) => result_t {
                    is_ok: false,
                    error_code,
                },
            }
        }
    }

    type ResultValue = Value<Result<(), u32>, result_t>;

    #[test]
    fn take_and_return() {
        let cval = result_t {
            is_ok: false,
            error_code: 13,
        };
        let rval = ResultValue::take(cval.clone());
        assert_eq!(rval, Err(13));
        assert_eq!(ResultValue::return_val(rval), cval);
    }

    #[test]
    fn to_out_param() {
        let mut cval = mem::MaybeUninit::uninit();
        // SAFETY: arg_out is not NULL
        unsafe {
            ResultValue::to_out_param(Ok(()), cval.as_mut_ptr());
        }
        // SAFETY: to_out_param initialized cval
        assert_eq!(ResultValue::take(unsafe { cval.assume_init() }), Ok(()));
    }

    #[test]
    fn to_out_param_null() {
        // SAFETY: passing null results in no action
        unsafe {
            ResultValue::to_out_param(Ok(()), std::ptr::null_mut());
        }
    }

    #[test]
    fn to_out_param_nonnull() {
        let mut cval = mem::MaybeUninit::uninit();
        // SAFETY: arg_out is not NULL
        unsafe {
            ResultValue::to_out_param_nonnull(Ok(()), cval.as_mut_ptr());
        }
        // SAFETY: to_out_param initialized cval
        assert_eq!(ResultValue::take(unsafe { cval.assume_init() }), Ok(()));
    }

    #[test]
    #[should_panic]
    fn to_out_param_nonnull_null() {
        // SAFETY: well, it's not safe, that's why it panics!
        unsafe {
            ResultValue::to_out_param_nonnull(Ok(()), std::ptr::null_mut());
        }
    }
}
