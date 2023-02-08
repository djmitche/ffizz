use crate::{EmbeddedNulError, InvalidUTF8Error};
use ffizz_passby::OpaqueStruct;
use std::ffi::{CStr, CString, OsString};
use std::path::PathBuf;

/// A FzString carries a single string between Rust and C code, represented from the C side as
/// an opaque struct.
///
/// The two environments carry some different requirements: C generally requires that strings be
/// NUL-terminated, while Rust requires that strings be valid UTF-8.  Rust also permits NUL
/// characters in the middle of a string.
///
/// This type accepts whatever kind of data it receives without error, and converts -- potentially
/// with an error -- when output of a different kind is required.
///
/// A number of `From<T>` implementations are provided to convert from common Rust types. The
/// `fz_string_..` utility functions provide conversions from various string formats.
///
/// FzStrings also have a special "Null" state, similar to the None variant of Option.  For user
/// convenience, a NULL pointer is treated as a pointer to the Null variant wherever a pointer is
/// accepted.  Rust code should use the `_nonnull` methods where the Null variant is not allowed.
/// Note that the Null variant is not necessarily represented with an all-zero byte pattern.
///
/// A FzString points to allocated memory, and must be freed to avoid memory leaks.
#[derive(PartialEq, Eq, Debug)]
pub enum FzString<'a> {
    /// An un-set FzString.
    Null,
    /// An owned Rust string (not NUL-terminated, valid UTF-8).
    String(String),
    /// An owned C String (NUL-terminated, may contain invalid UTF-8).
    CString(CString),
    /// A borrowed C string.
    CStr(&'a CStr),
    /// An owned bunch of bytes (not NUL-terminated, may contain invalid UTF-8).
    Bytes(Vec<u8>),
}

/// fz_string_t represents a string suitable for use with this crate, as an opaque stack-allocated
/// value.
///
/// This value can contain either a string or a special "Null" variant indicating there is no
/// string.  When functions take a `fz_string_t*` as an argument, the NULL pointer is treated as
/// the Null variant.  Note that the Null variant is not necessarily represented as the zero value
/// of the struct.
///
/// # Safety
///
/// A fz_string_t must always be initialized before it is passed as an argument.  Functions
/// returning a `fz_string_t` return an initialized value.
///
/// Each initialized fz_string_t must be freed, either by calling fz_string_free or by
/// passing the string to a function which takes ownership of the string.
///
/// For a given fz_string_t value, API functions must not be called concurrently.  This includes
/// "read only" functions such as fz_string_content.
///
/// ```c
/// typedef struct fz_string_t {
///     uint64_t __reserved[4];
/// };
/// ```
#[repr(C)]
pub struct fz_string_t {
    // size for a determinant, pointer, length, and capacity; conservatively assuming
    // 64 bits for each, and assuring 64-bit alignment.
    __reserved: [u64; 4],
}

impl OpaqueStruct for FzString<'_> {
    type CType = fz_string_t;

    fn null_value() -> Self {
        FzString::Null
    }
}

impl Default for FzString<'_> {
    fn default() -> Self {
        FzString::Null
    }
}

impl<'a> FzString<'a> {
    /// Check if this is a Null FzString.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Convert this value to `&str`.
    ///
    /// If required, the FzString is converted in-place to a String variant. If this conversion
    /// fails because the content is not valid UTF-8, an error is returned.
    ///
    /// The Null FzString is represented as None.
    pub fn as_str(&mut self) -> Result<Option<&str>, InvalidUTF8Error> {
        // first, convert in-place from bytes
        if let FzString::Bytes(_) = self {
            self.bytes_to_string()?;
        }

        Ok(match self {
            FzString::CString(cstring) => {
                Some(cstring.as_c_str().to_str().map_err(|_| InvalidUTF8Error)?)
            }
            FzString::CStr(cstr) => Some(cstr.to_str().map_err(|_| InvalidUTF8Error)?),
            FzString::String(ref string) => Some(string.as_ref()),
            FzString::Bytes(_) => unreachable!(), // handled above
            FzString::Null => None,
        })
    }

    /// Convert this FzString, assuming it is not Null, into `&str`.
    ///
    /// This is a simple wrapper that will panic on the Null variant.  This is useful when
    /// the C API prohibits NULL.
    pub fn as_str_nonnull(&mut self) -> Result<&str, InvalidUTF8Error> {
        self.as_str()
            .map(|opt| opt.expect("unexpected NULL string"))
    }

    /// Convert this value to a CStr: a slice of bytes containing a valid, NUL-terminated C string.
    ///
    /// If required, the FzString is converted in-place to a CString variant. If this conversion
    /// fails because the content contains embedded NUL characters, an error is returned.
    ///
    /// The Null FzString is represented as None.
    pub fn as_cstr(&mut self) -> Result<Option<&CStr>, EmbeddedNulError> {
        // first, convert in-place from String or Bytes (neither of which have a NUL terminator)
        match self {
            FzString::String(_) => self.string_to_cstring()?,
            FzString::Bytes(_) => self.bytes_to_cstring()?,
            _ => {}
        }

        Ok(match self {
            FzString::CString(cstring) => Some(cstring.as_c_str()),
            FzString::CStr(cstr) => Some(cstr),
            FzString::String(_) => unreachable!(), // handled above
            FzString::Bytes(_) => unreachable!(),  // handled above
            FzString::Null => None,
        })
    }

    /// Convert this FzString, assuming it is not Null, into a CStr.
    ///
    /// This is a simple wrapper that will panic on the Null variant.  This is useful when
    /// the C API prohibits NULL.
    pub fn as_cstr_nonnull(&mut self) -> Result<&CStr, EmbeddedNulError> {
        self.as_cstr()
            .map(|opt| opt.expect("unexpected NULL string"))
    }

    /// Consume this FzString and return an equivalent String.
    ///
    /// As with `as_str`, the FzString is converted in-place, and this conversion can fail.  In the
    /// failure case, the original data is lost.
    ///
    /// The Null varaiant is represented as None.
    pub fn into_string(mut self) -> Result<Option<String>, InvalidUTF8Error> {
        // first, convert in-place from bytes
        if let FzString::Bytes(_) = self {
            self.bytes_to_string()?;
        }

        Ok(match self {
            FzString::CString(cstring) => {
                Some(cstring.into_string().map_err(|_| InvalidUTF8Error)?)
            }
            FzString::CStr(cstr) => Some(
                cstr.to_str()
                    .map(|s| s.to_string())
                    .map_err(|_| InvalidUTF8Error)?,
            ),
            FzString::String(string) => Some(string),
            FzString::Bytes(_) => unreachable!(), // handled above
            FzString::Null => None,
        })
    }

    /// Consume this FzString, assuming it is not Null, and return an equivalent String.
    ///
    /// This is a simple wrapper that will panic on the Null variant.  This is useful when
    /// the C API prohibits NULL.
    pub fn into_string_nonnull(self) -> Result<String, InvalidUTF8Error> {
        self.into_string()
            .map(|opt| opt.expect("unexpected NULL string"))
    }

    /// Consume this FzString and return an equivalent PathBuf.
    ///
    /// As with `as_str`, the FzString is converted in-place, and this conversion can fail.  In the
    /// failure case, the original data is lost.
    ///
    /// The Null varaiant is represented as None.
    pub fn into_path_buf(self) -> Result<Option<PathBuf>, std::str::Utf8Error> {
        #[cfg(unix)]
        let path: Option<OsString> = {
            // on UNIX, we can use the bytes directly, without requiring that they
            // be valid UTF-8.
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;
            self.as_bytes()
                .map(|bytes| OsStr::from_bytes(bytes).to_os_string())
        };
        #[cfg(windows)]
        let path: Option<OsString> = {
            // on Windows, we assume the filename is valid Unicode, so it can be
            // represented as UTF-8.
            self.into_string()?.map(|s| OsString::from(s))
        };
        Ok(path.map(|p| p.into()))
    }

    /// Consume this FzString, assuming it is not Null, and return an equivalent PathBuf.
    ///
    /// This is a simple wrapper that will panic on the Null variant.  This is useful when
    /// the C API prohibits NULL.
    pub fn into_path_buf_nonnull(self) -> Result<PathBuf, std::str::Utf8Error> {
        self.into_path_buf()
            .map(|opt| opt.expect("unexpected NULL string"))
    }

    /// Get the slice of bytes representing the content of this value, not including any NUL
    /// terminator.
    ///
    /// Any variant can be represented as a byte slice, so this method does not mutate the
    /// FzString and cannot fail.
    ///
    /// The Null variant is represented as None.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            FzString::CString(cstring) => Some(cstring.as_bytes()),
            FzString::CStr(cstr) => Some(cstr.to_bytes()),
            FzString::String(string) => Some(string.as_bytes()),
            FzString::Bytes(bytes) => Some(bytes.as_ref()),
            FzString::Null => None,
        }
    }

    /// Get the slice of bytes representing the content of this value, not including any NUL
    /// terminator, panicing if this is the Null Variant.
    ///
    /// This is a simple wrapper that will panic on the Null variant.  This is useful when
    /// the C API prohibits NULL.
    pub fn as_bytes_nonnull(&self) -> &[u8] {
        self.as_bytes().expect("unexpected NULL string")
    }

    /// Call the contained function with a shared reference to the FzString.
    ///
    /// This is a wrapper around `ffizz_passby::OpaqueStruct::with_ref`.
    ///
    /// # Safety
    ///
    /// * fzstr must be NULL or point to a valid fz_string_t value
    /// * no other thread may mutate the value pointed to by fzstr until with_ref returns.
    #[inline]
    pub unsafe fn with_ref<T, F: Fn(&FzString) -> T>(fzstr: *const fz_string_t, f: F) -> T {
        unsafe { <Self as OpaqueStruct>::with_ref(fzstr, f) }
    }

    /// Call the contained function with an exclusive reference to the FzString.
    ///
    /// This is a wrapper around `ffizz_passby::OpaqueStruct::with_ref_mut`.
    ///
    /// # Safety
    ///
    /// * fzstr must be NULL or point to a valid `fz_string_t` value
    /// * no other thread may access the value pointed to by `fzstr` until `with_ref_mut` returns.
    #[inline]
    pub unsafe fn with_ref_mut<T, F: Fn(&mut FzString) -> T>(fzstr: *mut fz_string_t, f: F) -> T {
        unsafe { <Self as OpaqueStruct>::with_ref_mut(fzstr, f) }
    }

    /// Initialize the value pointed to fzstr with, "moving" it into the pointer.
    ///
    /// This is a wrapper around `ffizz_passby::OpaqueStruct::to_out_param`.
    ///
    /// If the pointer is NULL, the value is dropped.
    ///
    /// # Safety
    ///
    /// * if fzstr is not NULl, then it must be aligned for fz_string_t, and must have enough space
    ///   for fz_string_t.
    /// * ownership of the string is transfered to `*fzstr` or dropped.
    #[inline]
    pub unsafe fn to_out_param(self, fzstr: *mut fz_string_t) {
        unsafe { <Self as OpaqueStruct>::to_out_param(self, fzstr) }
    }

    /// Initialize the value pointed to fzstr with, "moving" it into the pointer.
    ///
    /// This is a wrapper around `ffizz_passby::OpaqueStruct::to_out_param_nonnull`.
    ///
    /// If the pointer is NULL, this method will panic.  Use this when the C API requires that the
    /// pointer be non-NULL.
    ///
    /// # Safety
    ///
    /// * fzstr must not be NULL, must be aligned for fz_string_t, and must have enough space for
    ///   fz_string_t.
    /// * ownership of the string is transfered to `*fzstr`.
    #[inline]
    pub unsafe fn to_out_param_nonnull(self, fzstr: *mut fz_string_t) {
        unsafe { <Self as OpaqueStruct>::to_out_param_nonnull(self, fzstr) }
    }

    /// Return a `fz_string_t` transferring ownership out of the function.
    ///
    /// This is a wrapper around `ffizz_passby::OpaqueStruct::return_val`.
    ///
    /// # Safety
    ///
    /// * to avoid a leak, ownership of the value must eventually be returned to Rust.
    #[inline]
    pub unsafe fn return_val(self) -> fz_string_t {
        unsafe { <Self as OpaqueStruct>::return_val(self) }
    }

    /// Take a `fz_string_t` by value and return an owned `FzString`.
    ///
    /// This is a wrapper around `ffizz_passby::OpaqueStruct::take`.
    ///
    /// This method is intended for C API functions that take a string by value and are
    /// documented as taking ownership of the value.  However, this means that C retains
    /// an expired "copy" of the value and could lead to use-after-free errors.
    ///
    /// Where compatible with the API design, prefer to use pointers in the C API and use
    /// [`FzString::take_ptr`] to ensure the old value is invalidated.
    ///
    /// # Safety
    ///
    /// * fzstr must be a valid `fz_string_t` value
    #[inline]
    pub unsafe fn take(fzstr: fz_string_t) -> Self {
        unsafe { <Self as OpaqueStruct>::take(fzstr) }
    }

    /// Take a pointer to a CType and return an owned value.
    ///
    /// This is a wrapper around `ffizz_passby::OpaqueStruct::take_ptr`.
    ///
    /// This is intended for C API functions that take a value by reference (pointer), but still
    /// "take ownership" of the value.  It leaves behind an invalid value, where any non-padding
    /// bytes of the Rust type are zeroed.  This makes use-after-free errors in the C code more
    /// likely to crash instead of silently working.  Which is about as good as it gets in C.
    ///
    /// Do _not_ pass a pointer to a Rust value to this function:
    ///
    /// ```ignore
    /// let rust_value = RustType::take_ptr(&mut c_value); // BAD!
    /// ```
    ///
    /// This creates undefined behavior as Rust will assume `c_value` is still initialized. Use
    /// `take` in this situation.
    ///
    /// # Safety
    ///
    /// * fzstr must be NULL or point to a valid fz_string_t value.
    /// * the memory pointed to by fzstr is uninitialized when this function returns.
    #[inline]
    pub unsafe fn take_ptr(fzstr: *mut fz_string_t) -> Self {
        unsafe { <Self as OpaqueStruct>::take_ptr(fzstr) }
    }

    /// Convert the FzString, in place, from a Bytes to String variant, returning None if
    /// the bytes do not contain valid UTF-8.
    fn bytes_to_string(&mut self) -> Result<(), InvalidUTF8Error> {
        if let FzString::Bytes(bytes) = self {
            // first, check for invalid UTF-8
            if std::str::from_utf8(bytes).is_err() {
                return Err(InvalidUTF8Error);
            }
            // take ownership of the bytes Vec
            let bytes = std::mem::take(bytes);
            // SAFETY: we just checked this..
            let string = unsafe { String::from_utf8_unchecked(bytes) };
            *self = FzString::String(string);
            Ok(())
        } else {
            unreachable!()
        }
    }

    /// Convert the FxString, in place, from a Bytes to CString variant, returning None if the
    /// string contains embedded NULs.
    ///
    /// Panics if self is not Bytes.
    fn bytes_to_cstring(&mut self) -> Result<(), EmbeddedNulError> {
        if let FzString::Bytes(bytes) = self {
            // first, check for NUL bytes within the sequence
            if has_nul_bytes(bytes) {
                return Err(EmbeddedNulError);
            }
            // take ownership of the bytes Vec
            let bytes = std::mem::take(bytes);
            // SAFETY: we just checked for NUL bytes
            let cstring = unsafe { CString::from_vec_unchecked(bytes) };
            *self = FzString::CString(cstring);
            Ok(())
        } else {
            unreachable!()
        }
    }

    /// Convert the FzString, in place, from a String to CString variant, returning None if the
    /// string contains embedded NULs.
    ///
    /// Panics if self is not String.
    fn string_to_cstring(&mut self) -> Result<(), EmbeddedNulError> {
        if let FzString::String(string) = self {
            // first, check for NUL bytes within the sequence
            if has_nul_bytes(string.as_bytes()) {
                return Err(EmbeddedNulError);
            }
            // take ownership of the string
            let string = std::mem::take(string);
            // SAFETY: we just checked for NUL bytes
            let cstring = unsafe { CString::from_vec_unchecked(string.into_bytes()) };
            *self = FzString::CString(cstring);
            Ok(())
        } else {
            unreachable!()
        }
    }
}

impl From<String> for FzString<'static> {
    fn from(string: String) -> FzString<'static> {
        FzString::String(string)
    }
}

impl From<&str> for FzString<'static> {
    fn from(string: &str) -> FzString<'static> {
        FzString::String(string.to_string())
    }
}

impl From<Vec<u8>> for FzString<'static> {
    fn from(bytes: Vec<u8>) -> FzString<'static> {
        FzString::Bytes(bytes)
    }
}

impl From<&[u8]> for FzString<'static> {
    fn from(bytes: &[u8]) -> FzString<'static> {
        FzString::Bytes(bytes.to_vec())
    }
}

impl From<Option<String>> for FzString<'static> {
    fn from(string: Option<String>) -> FzString<'static> {
        match string {
            Some(string) => FzString::String(string),
            None => FzString::Null,
        }
    }
}

impl From<Option<&str>> for FzString<'static> {
    fn from(string: Option<&str>) -> FzString<'static> {
        match string {
            Some(string) => FzString::String(string.to_string()),
            None => FzString::Null,
        }
    }
}

impl From<Option<Vec<u8>>> for FzString<'static> {
    fn from(bytes: Option<Vec<u8>>) -> FzString<'static> {
        match bytes {
            Some(bytes) => FzString::Bytes(bytes),
            None => FzString::Null,
        }
    }
}

impl From<Option<&[u8]>> for FzString<'static> {
    fn from(bytes: Option<&[u8]>) -> FzString<'static> {
        match bytes {
            Some(bytes) => FzString::Bytes(bytes.to_vec()),
            None => FzString::Null,
        }
    }
}

fn has_nul_bytes(bytes: &[u8]) -> bool {
    bytes.iter().any(|c| *c == b'\x00')
}

#[cfg(test)]
mod test {
    use super::*;

    const INVALID_UTF8: &[u8] = b"abc\xf0\x28\x8c\x28";

    fn make_cstring() -> FzString<'static> {
        FzString::CString(CString::new("a string").unwrap())
    }

    fn make_cstr() -> FzString<'static> {
        let cstr = CStr::from_bytes_with_nul(b"a string\x00").unwrap();
        FzString::CStr(cstr)
    }

    fn make_string() -> FzString<'static> {
        "a string".into()
    }

    fn make_string_with_nul() -> FzString<'static> {
        "a \x00 nul!".into()
    }

    fn make_invalid_bytes() -> FzString<'static> {
        INVALID_UTF8.into()
    }

    fn make_nul_bytes() -> FzString<'static> {
        (&b"abc\x00123"[..]).into()
    }

    fn make_bytes() -> FzString<'static> {
        (&b"bytes"[..]).into()
    }

    fn make_null() -> FzString<'static> {
        FzString::Null
    }

    fn cstr(s: &str) -> &CStr {
        CStr::from_bytes_with_nul(s.as_bytes()).unwrap()
    }

    // as_str

    #[test]
    fn as_str_cstring() {
        assert_eq!(make_cstring().as_str().unwrap(), Some("a string"));
    }

    #[test]
    fn as_str_cstr() {
        assert_eq!(make_cstr().as_str().unwrap(), Some("a string"));
    }

    #[test]
    fn as_str_string() {
        assert_eq!(make_string().as_str().unwrap(), Some("a string"));
    }

    #[test]
    fn as_str_string_with_nul() {
        assert_eq!(make_string_with_nul().as_str().unwrap(), Some("a \x00 nul!"));
    }

    #[test]
    fn as_str_invalid_bytes() {
        assert_eq!(make_invalid_bytes().as_str().unwrap_err(), InvalidUTF8Error);
    }

    #[test]
    fn as_str_nul_bytes() {
        assert_eq!(make_nul_bytes().as_str().unwrap(), Some("abc\x00123"));
    }

    #[test]
    fn as_str_valid_bytes() {
        assert_eq!(make_bytes().as_str().unwrap(), Some("bytes"));
    }

    #[test]
    fn as_str_null() {
        assert!(make_null().as_str().unwrap().is_none());
    }

    #[test]
    fn as_str_nonnull_string() {
        assert_eq!(make_string().as_str_nonnull().unwrap(), "a string");
    }

    #[test]
    #[should_panic]
    fn as_str_nonnull_null() {
        let _res = make_null().as_str_nonnull();
    }

    // as_cstr

    #[test]
    fn as_cstr_cstring() {
        assert_eq!(make_cstring().as_cstr().unwrap(), Some(cstr("a string\x00")));
    }

    #[test]
    fn as_cstr_cstr() {
        assert_eq!(make_cstr().as_cstr().unwrap(), Some(cstr("a string\x00")));
    }

    #[test]
    fn as_cstr_string() {
        assert_eq!(make_string().as_cstr().unwrap(), Some(cstr("a string\x00")));
    }

    #[test]
    fn as_cstr_string_with_nul() {
        assert_eq!(
            make_string_with_nul().as_cstr().unwrap_err(),
            EmbeddedNulError
        );
    }

    #[test]
    fn as_cstr_invalid_bytes() {
        let expected = CString::new(INVALID_UTF8).unwrap();
        assert_eq!(
            make_invalid_bytes().as_cstr().unwrap(),
            Some(expected.as_c_str())
        );
    }

    #[test]
    fn as_cstr_nul_bytes() {
        assert_eq!(make_nul_bytes().as_cstr().unwrap_err(), EmbeddedNulError);
    }

    #[test]
    fn as_cstr_valid_bytes() {
        assert_eq!(make_bytes().as_cstr().unwrap(), Some(cstr("bytes\x00")));
    }

    #[test]
    fn as_cstr_null() {
        assert_eq!(make_null().as_cstr().unwrap(), None);
    }

    #[test]
    fn as_cstr_nonnull_string() {
        assert_eq!(make_string().as_cstr_nonnull().unwrap(), cstr("a string\x00"));
    }

    #[test]
    #[should_panic]
    fn as_cstr_nonnull_null() {
        let _res = make_null().as_cstr_nonnull();
    }

    // into_string

    #[test]
    fn into_string_cstring() {
        assert_eq!(
            make_cstring().into_string().unwrap(),
            Some(String::from("a string"))
        );
    }

    #[test]
    fn into_string_cstr() {
        assert_eq!(
            make_cstr().into_string().unwrap(),
            Some(String::from("a string"))
        );
    }

    #[test]
    fn into_string_string() {
        assert_eq!(
            make_string().into_string().unwrap(),
            Some(String::from("a string"))
        );
    }

    #[test]
    fn into_string_string_with_nul() {
        assert_eq!(
            make_string_with_nul().into_string().unwrap(),
            Some(String::from("a \x00 nul!"))
        )
    }

    #[test]
    fn into_string_invalid_bytes() {
        assert_eq!(
            make_invalid_bytes().into_string().unwrap_err(),
            InvalidUTF8Error
        );
    }

    #[test]
    fn into_string_nul_bytes() {
        assert_eq!(
            make_nul_bytes().into_string().unwrap(),
            Some(String::from("abc\x00123"))
        );
    }

    #[test]
    fn into_string_valid_bytes() {
        assert_eq!(
            make_bytes().into_string().unwrap(),
            Some(String::from("bytes"))
        );
    }

    #[test]
    fn into_string_null() {
        assert_eq!(make_null().into_string().unwrap(), None);
    }

    #[test]
    fn into_string_nonnull_string() {
        assert_eq!(
            make_string().into_string_nonnull().unwrap(),
            String::from("a string")
        );
    }

    #[test]
    #[should_panic]
    fn into_string_nonnull_null() {
        let _res = make_null().into_string_nonnull();
    }

    // into_path_buf

    #[test]
    fn into_path_buf_cstring() {
        assert_eq!(
            make_cstring().into_path_buf().unwrap(),
            Some(PathBuf::from("a string"))
        );
    }

    #[test]
    fn into_path_buf_cstr() {
        assert_eq!(
            make_cstr().into_path_buf().unwrap(),
            Some(PathBuf::from("a string"))
        );
    }

    #[test]
    fn into_path_buf_string() {
        assert_eq!(
            make_string().into_path_buf().unwrap(),
            Some(PathBuf::from("a string"))
        );
    }

    #[test]
    fn into_path_buf_string_with_nul() {
        assert_eq!(
            make_string_with_nul().into_path_buf().unwrap(),
            Some(PathBuf::from("a \x00 nul!"))
        )
    }

    #[test]
    fn into_path_buf_invalid_bytes() {
        #[cfg(windows)] // windows filenames are unicode
        assert!(make_invalid_bytes().into_path_buf().is_err());
        #[cfg(unix)] // UNIX doesn't care
        assert!(make_invalid_bytes().into_path_buf().is_ok());
    }

    #[test]
    fn into_path_buf_nul_bytes() {
        assert_eq!(
            make_nul_bytes().into_path_buf().unwrap(),
            Some(PathBuf::from("abc\x00123"))
        );
    }

    #[test]
    fn into_path_buf_valid_bytes() {
        assert_eq!(
            make_bytes().into_path_buf().unwrap(),
            Some(PathBuf::from("bytes"))
        );
    }

    #[test]
    fn into_path_buf_null() {
        assert_eq!(make_null().into_path_buf().unwrap(), None);
    }

    #[test]
    fn into_path_buf_nonnull_string() {
        assert_eq!(
            make_string().into_path_buf_nonnull().unwrap(),
            PathBuf::from("a string")
        );
    }

    #[test]
    #[should_panic]
    fn into_path_buf_nonnull_null() {
        let _res = make_null().into_path_buf_nonnull();
    }

    // as_bytes

    #[test]
    fn as_bytes_cstring() {
        assert_eq!(make_cstring().as_bytes().unwrap(), b"a string");
    }

    #[test]
    fn as_bytes_cstr() {
        assert_eq!(make_cstr().as_bytes().unwrap(), b"a string");
    }

    #[test]
    fn as_bytes_string() {
        assert_eq!(make_string().as_bytes().unwrap(), b"a string");
    }

    #[test]
    fn as_bytes_string_with_nul() {
        assert_eq!(make_string_with_nul().as_bytes().unwrap(), b"a \x00 nul!");
    }

    #[test]
    fn as_bytes_invalid_bytes() {
        assert_eq!(make_invalid_bytes().as_bytes().unwrap(), INVALID_UTF8);
    }

    #[test]
    fn as_bytes_null_bytes() {
        assert_eq!(make_nul_bytes().as_bytes().unwrap(), b"abc\x00123");
    }

    #[test]
    fn as_bytes_null() {
        assert_eq!(make_null().as_bytes(), None);
    }

    #[test]
    fn as_bytes_nonnul_string() {
        assert_eq!(make_string().as_bytes_nonnull(), b"a string");
    }

    #[test]
    #[should_panic]
    fn as_bytes_nonnull_null() {
        let _res = make_null().as_bytes_nonnull();
    }

    // From<..>

    #[test]
    fn from_string() {
        assert_eq!(
            FzString::from(String::from("hello")),
            FzString::String(String::from("hello"))
        );
    }

    #[test]
    fn from_str() {
        assert_eq!(
            FzString::from("hello"),
            FzString::String(String::from("hello"))
        );
    }

    #[test]
    fn from_vec() {
        assert_eq!(FzString::from(vec![1u8, 2u8]), FzString::Bytes(vec![1, 2]));
    }

    #[test]
    fn from_bytes() {
        assert_eq!(FzString::from(INVALID_UTF8), make_invalid_bytes());
    }

    #[test]
    fn from_option_string() {
        assert_eq!(FzString::from(None as Option<String>), FzString::Null);
        assert_eq!(
            FzString::from(Some(String::from("hello"))),
            FzString::String(String::from("hello")),
        );
    }

    #[test]
    fn from_option_str() {
        assert_eq!(FzString::from(None as Option<&str>), FzString::Null);
        assert_eq!(
            FzString::from(Some("hello")),
            FzString::String(String::from("hello")),
        );
    }

    #[test]
    fn from_option_vec() {
        assert_eq!(FzString::from(None as Option<Vec<u8>>), FzString::Null);
        assert_eq!(
            FzString::from(Some(vec![1u8, 2u8])),
            FzString::Bytes(vec![1, 2])
        );
    }

    #[test]
    fn from_option_bytes() {
        assert_eq!(FzString::from(None as Option<&[u8]>), FzString::Null);
        assert_eq!(
            FzString::from(Some(INVALID_UTF8)),
            FzString::Bytes(INVALID_UTF8.into())
        );
    }
}
