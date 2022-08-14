#![warn(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use ffizz_passby::PassByValue;
use libc::c_char;
use std::ffi::CStr;
use uuid::Uuid;

// NOTE: this must be a simple constant so that cbindgen can evaluate it
/// Length, in bytes, of the string representation of a UUID (without NUL terminator)
pub const UUID_STRING_BYTES: usize = 36;

/// uuid_t contains a UUID, represented as big-endian bytes.
///
/// cbindgen:field-names=[bytes]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct uuid_t([u8; 16]);

impl PassByValue for uuid_t {
    type RustType = Uuid;

    unsafe fn from_ctype(self) -> Self::RustType {
        // SAFETY:
        //  - any 16-byte value is a valid Uuid
        Uuid::from_bytes(self.0)
    }

    fn as_ctype(arg: Uuid) -> Self {
        uuid_t(*arg.as_bytes())
    }
}

/// Create a new, randomly-generated UUID.
#[no_mangle]
pub unsafe extern "C" fn uuid_new_v4() -> uuid_t {
    // SAFETY:
    // - value is not allocated
    unsafe { uuid_t::return_val(Uuid::new_v4()) }
}

/// Create a new UUID with the nil value.
#[no_mangle]
pub unsafe extern "C" fn uuid_nil() -> uuid_t {
    // SAFETY:
    // - value is not allocated
    unsafe { uuid_t::return_val(Uuid::nil()) }
}

/// Get the version of the given UUID.
#[no_mangle]
pub unsafe extern "C" fn uuid_version(uuid: uuid_t) -> usize {
    // SAFETY:
    //  - tcuuid is a valid uuid_t (all byte patterns are valid)
    let uuid: Uuid = unsafe { uuid_t::val_from_arg(uuid) };
    uuid.get_version_num()
}

/// Write the string representation of a uuid_t into the given buffer, which must be
/// at least UUID_STRING_BYTES long.  No NUL terminator is added.
///
/// # Safety
///
/// * buf must point to at least UUID_STRING_BYTES of valid memory.
#[no_mangle]
pub unsafe extern "C" fn uuid_to_buf(tcuuid: uuid_t, buf: *mut c_char) {
    debug_assert!(!buf.is_null());
    // SAFETY:
    //  - buf is valid for len bytes (by C convention)
    //  - (no alignment requirements for a byte slice)
    //  - content of buf will not be mutated during the lifetime of this slice (lifetime
    //    does not outlive this function call)
    //  - the length of the buffer is less than isize::MAX (see docstring)
    let buf: &mut [u8] =
        unsafe { std::slice::from_raw_parts_mut(buf as *mut u8, UUID_STRING_BYTES) };
    // SAFETY:
    //  - tcuuid is a valid uuid_t (all byte patterns are valid)
    let uuid: Uuid = unsafe { uuid_t::val_from_arg(tcuuid) };
    uuid.as_hyphenated().encode_lower(buf);
}

/// Parse the given string as a UUID.  Returns false on parse failure or if the given
/// string is not valid.
///
/// # Safety
///
/// * s must be non-NULL and point to a valid NUL-terminated string.
/// * s is not modified by this function.
/// * uuid_out must be non-NULL and point to a valid memory location for a uuid_t.
#[no_mangle]
pub unsafe extern "C" fn uuid_from_str(s: *const c_char, uuid_out: *mut uuid_t) -> bool {
    debug_assert!(!s.is_null());
    debug_assert!(!uuid_out.is_null());
    // SAFETY:
    //  - s is valid (see docstring)
    let s = unsafe { CStr::from_ptr(s) };
    if let Ok(s) = s.to_str() {
        if let Ok(u) = Uuid::parse_str(s) {
            // SAFETY:
            //  - uuid_out is not NULL (see docstring)
            //  - alignment is not required
            unsafe { uuid_t::val_to_arg_out(u, uuid_out) };
            return true;
        }
    }
    false
}

fn main() {
    let u = unsafe { uuid_new_v4() };
    assert_eq!(unsafe { uuid_version(u) }, 4);

    let u = unsafe { uuid_nil() };
    assert_eq!(unsafe { uuid_version(u) }, 0);

    let mut buf = [0u8; UUID_STRING_BYTES];
    unsafe { uuid_to_buf(u, buf.as_mut_ptr() as *mut c_char) };
    assert_eq!(
        std::str::from_utf8(&buf[..]).expect("invalid utf-8"),
        "00000000-0000-0000-0000-000000000000"
    );

    let mut u = unsafe { uuid_nil() };
    assert!(unsafe {
        uuid_from_str(
            // (cheating a little, by creating a C string with explicit trailing NUL)
            "d9c5d004-1bf4-11ed-861d-0242ac120002\0".as_ptr() as *const c_char,
            &mut u as *mut uuid_t,
        )
    });
    assert_eq!(unsafe { uuid_version(u) }, 1);
}
