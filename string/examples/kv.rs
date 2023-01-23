#![warn(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use ffizz_passby::{OpaqueStruct, PassByPointer};
use ffizz_string::{fz_string_t as kvstore_string_t, FzString};
use std::collections::HashMap;

ffizz_header::snippet! {
#[ffizz(name="header", order=0)]
/// This library implements a simple in-memory key-value store.
}

pub struct Store {
    map: HashMap<String, String>,
}

impl Store {
    fn new() -> Store {
        Store {
            map: HashMap::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|v| &**v)
    }

    fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.map.insert(key.into(), value.into());
    }

    fn del(&mut self, key: &str) {
        self.map.remove(key);
    }
}

#[ffizz_header::item]
#[ffizz(order = 10)]
/// This opaque pointer type represents a key-value store.
///
/// # Safety
///
/// In a multi-threaded program, kvstore_t values may be passed from thread to thread, but
/// _must not_ be accessed concurrently from multiple threads.
///
/// Each kvstore_t created with `kvstore_new` must later be freed with `kvstore_free`, and
/// once freed must not be used again.
///
/// Keys and values must be valid UTF-8 strings.
///
/// ```c
/// typedef struct kvstore_t kvstore_t;
/// ```
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct kvstore_t(pub Store);

impl PassByPointer for kvstore_t {}

#[ffizz_header::item]
#[ffizz(order = 20)]
/// Create a new kvstore_t.
///
/// # Safety
///
/// The returned kvstore_t must be freed with kvstore_free.
///
/// ```c
/// kvstore_t *kvstore_new();
/// ```
#[no_mangle]
pub unsafe extern "C" fn kvstore_new() -> *mut kvstore_t {
    let store = Store::new();
    // SAFETY: function docs indicate value must be freed
    unsafe { kvstore_t(store).return_ptr() }
}

#[ffizz_header::item]
#[ffizz(order = 21)]
/// Free a kvstore_t.
///
/// # Safety
///
/// The argument must be non-NULL and point to a valid kvstore_t. After this call it is no longer
/// valid and must not be used.
///
/// ```c
/// void kvstore_free(*kvstore_t);
/// ```
#[no_mangle]
pub unsafe extern "C" fn kvstore_free(store: *mut kvstore_t) {
    // SAFETY:
    //  - store is valid and not NULL (see docstring)
    //  - caller will not use store after this call (see docstring)
    let store = unsafe { kvstore_t::take_from_ptr_arg(store) };
    drop(store); // (Rust would do this anyway, but let's be explicit)
}

#[ffizz_header::item]
#[ffizz(order = 30)]
/// Get a value from the kvstore_t.  If the value is not found, the returned string is a Null
/// variant (test with `kvstore_string_is_null`).
///
/// # Safety
///
/// The store must be non-NULL and point to a valid kvstore_t.
///
/// The key argument must be a valid kvstore_string_t.  The caller must free both the key and the
/// returned string, if any.
/// ```c
/// fz_string_t kvstore_get(kvstore_t *store, kvstore_string_t *key);
/// ```
#[no_mangle]
pub unsafe extern "C" fn kvstore_get(
    store: *mut kvstore_t,
    key: *mut kvstore_string_t,
) -> kvstore_string_t {
    // SAFETY:
    // - store is not NULL and valid (see docstring)
    // - store is valid for the life of this function (documented as not threadsafe)
    // - store will not be accessed during the life of this function (documented as not threadsafe)
    let store = &unsafe { kvstore_t::from_ptr_arg_ref(store) }.0;
    // SAFETY:
    //  - key must be a valid kvstore_string_t (docstring)
    //  - key will not be accessed concurrency (type docstring)
    match unsafe {
        FzString::with_ref_mut(key, |key| {
            if let Ok(Some(key)) = key.as_str() {
                store.get(key)
            } else {
                None // Null key or invalid UTF-8 looks the same as key-not-found
            }
        })
    } {
        // SAFETY:
        //  - the caller will free the returned value (see docstring)
        Some(val) => unsafe { FzString::return_val(FzString::String(val.to_string())) },
        // SAFETY:
        //  - the caller will free the returned value (see docstring)
        None => unsafe { FzString::return_val(FzString::Null) },
    }
}

#[ffizz_header::item]
#[ffizz(order = 30)]
/// Set a value in the kvstore_t, consuming the key and value.  Returns false on error.
///
/// # Safety
///
/// The store must be non-NULL and point to a valid kvstore_t.
///
/// The key and value must both be valid kvstore_string_t values, must not be otherwise accessed
/// while this function executes, and are invalid when this function returns.
///
/// # Note
///
/// The kvstore API sometimes invalidates its string arguments and sometimes leaves that
/// reponsibility to the caller, which could lead to confusion for users of the library. It's done
/// here for example purposes only!
///
/// ```c
/// bool kvstore_set(kvstore *store, kvstore_string_t *key, kvstore_string_t *value);
/// ```
#[no_mangle]
pub unsafe extern "C" fn kvstore_set(
    store: *mut kvstore_t,
    key: *mut kvstore_string_t,
    val: *mut kvstore_string_t,
) -> bool {
    // SAFETY:
    // - store is not NULL and valid (see docstring)
    // - store is valid for the life of this function (documented as not threadsafe)
    // - store will not be accessed during the life of this function (documented as not threadsafe)
    let store = &mut unsafe { kvstore_t::from_ptr_arg_ref_mut(store) }.0;
    // SAFETY:
    //  - key/val are valid kvstore_string_t's (see docstring)
    //  - key/val are not accessed concurrently (type docstring)
    //  - key/val are not uesd after function returns (see docstring)
    let (key, val) = unsafe { (FzString::take_ptr(key), FzString::take_ptr(val)) };

    if let Ok(Some(key)) = key.into_string() {
        if let Ok(Some(val)) = val.into_string() {
            store.set(key, val);
            return true;
        }
    }
    false
}

#[ffizz_header::item]
#[ffizz(order = 30)]
/// Delete a value from the kvstore_t.  Returns false on error.
///
/// # Safety
///
/// The store must be non-NULL and point to a valid kvstore_t.
///
/// The key must be a valid kvstore_string_t, must not be otherwise accessed while this function
/// executes, and will remain valid after this function returns.
/// ```c
/// bool kvstore_del(kvstore *store, kvstore_string_t *key);
/// ```
#[no_mangle]
pub unsafe extern "C" fn kvstore_del(store: *mut kvstore_t, key: *mut kvstore_string_t) -> bool {
    // SAFETY:
    //  - key must be a valid kvstore_string_t (docstring)
    //  - key will not be accessed concurrency (type docstring)
    unsafe {
        FzString::with_ref_mut(key, move |key| {
            // SAFETY:
            // - store is not NULL and valid (see docstring)
            // - store is valid for the life of this function (documented as not threadsafe)
            // - store will not be accessed during the life of this function (documented as not threadsafe)
            let store = &mut unsafe { kvstore_t::from_ptr_arg_ref_mut(store) }.0;

            if let Ok(Some(key)) = key.as_str() {
                store.del(key);
                true
            } else {
                false
            }
        })
    }
}

ffizz_header::snippet! {
#[ffizz(name="kvstore_string_t", order=100)]
/// kvstore_string_t represents a string suitable for use with kvstore, as an opaque
/// stack-allocated value.
///
/// This value can contain either a string or a special "Null" variant indicating there is no
/// string.  When functions take a `kvstore_string_t*` as an argument, the NULL pointer is treated as
/// the Null variant.  Note that the Null variant is not necessarily represented as the zero value
/// of the struct.
///
/// # Safety
///
/// A kvstore_string_t must always be initialized before it is passed as an argument.  Functions
/// returning a `kvstore_string_t` return an initialized value.
///
/// Each initialized kvstore_string_t must be freed, either by calling kvstore_string_free or by
/// passing the string to a function which takes ownership of the string.
///
/// For a given kvstore_string_t value, API functions must not be called concurrently.  This includes
/// "read only" functions such as kvstore_string_content.
///
/// ```c
/// typedef struct kvstore_string_t {
///     uint64_t __reserved[4];
/// };
/// ```
}

// re-export some of the kvstore_string_* as kvstore_string_*

#[ffizz_header::item]
#[ffizz(order = 110)]
/// Create a new `kvstore_string_t` by cloning the content of the given C string.  The resulting `fz_string_t`
/// is independent of the given string.
///
/// # Safety
///
/// The given pointer must not be NULL.
///
/// ```c
/// kvstore_string_t kvstore_string_clone(const char *);
/// ```
pub use ffizz_string::fz_string_clone as kvstore_string_clone;

#[ffizz_header::item]
#[ffizz(order = 110)]
/// Get the content of the string as a regular C string.
///
/// A string contianing NUL bytes will result in a NULL return value.  In general, prefer
/// `kvstore_string_content_with_len` except when it's certain that the string is NUL-free.
///
/// The Null variant also results in a NULL return value.
///
/// This function takes the kvstore_string_t by pointer because it may be modified in-place to add a NUL
/// terminator.  The pointer must not be NULL.
///
/// # Safety
///
/// The returned string is "borrowed" and remains valid only until the kvstore_string_t is freed or
/// passed to any other API function.
pub use ffizz_string::fz_string_content as kvstore_string_content;

#[ffizz_header::item]
#[ffizz(order = 110)]
/// Free a kvstore_string_t.
///
/// # Safety
///
/// The string must not be used after this function returns, and must not be freed more than once.
/// It is safe to free Null-variant strings.
///
/// ```c
/// kvstore_string_free(kvstore_string_t *);
/// ```
pub use ffizz_string::fz_string_free as kvstore_string_free;

#[ffizz_header::item]
#[ffizz(order = 110)]
/// Determine whether the given kvstore_string_t is a Null variant.
///
/// ```c
/// bool kvstore_string_is_null(kvstore_string_t *);
/// ```
pub use ffizz_string::fz_string_is_null as kvstore_string_is_null;

// Calling a C API from Rust is tricky, and not what ffizz is about.  This section serves as a
// test of the example code above, with equivalent C code included in comments.
fn main() {
    // kvstore_t *store = kvstore_new();
    let store = unsafe { kvstore_new() };

    /// Clone a Rust string into an kvstore_string_t
    fn fzstr(s: &str) -> kvstore_string_t {
        use std::ffi::CString;
        let cstr = CString::new(s).unwrap();
        unsafe { kvstore_string_clone(cstr.as_ptr()) }
    }

    /// Get a Rust &str containing the data in an kvstore_string_t
    fn rstr(fzs: &mut kvstore_string_t) -> &str {
        use std::ffi::CStr;
        let content =
            unsafe { CStr::from_ptr(kvstore_string_content(fzs as *mut kvstore_string_t)) };
        content.to_str().unwrap()
    }

    // kvstore_string_t key = kvstore_string_clone("a-key");
    let mut key = fzstr("a-key");

    // kvstore_string_t val = kvstore_get(store, key)
    let mut val = unsafe { kvstore_get(store, &mut key as *mut kvstore_string_t) };

    // assert(kvstore_string_is_null(val));
    assert!(unsafe { kvstore_string_is_null(&val as *const kvstore_string_t) });

    // kvstore_string_free(val);
    unsafe { kvstore_string_free(&mut val as *mut kvstore_string_t) };

    // val = kvstore_string_clone("a-val");
    let mut val = fzstr("a-val");

    // assert(kvstore_set(store, key, val));
    assert!(unsafe {
        kvstore_set(
            store,
            &mut key as *mut kvstore_string_t,
            &mut val as *mut kvstore_string_t,
        )
    });

    // key = kvstore_string_clone("a-key");
    let mut key = fzstr("a-key");

    // val = kvstore_get(store, key)
    let mut val = unsafe { kvstore_get(store, &mut key as *mut kvstore_string_t) };

    // assert(0 == strcmp(kvstore_string_content(val), "a-val"));
    assert_eq!(rstr(&mut val), "a-val");

    // kvstore_string_free(val);
    unsafe { kvstore_string_free(&mut val as *mut kvstore_string_t) };

    // assert(kvstore_del(store, key));
    assert!(unsafe { kvstore_del(store, &mut key as *mut kvstore_string_t,) });

    // val = kvstore_get(store, key)
    let mut val = unsafe { kvstore_get(store, &mut key as *mut kvstore_string_t) };

    // assert(kvstore_string_is_null(val));
    assert!(unsafe { kvstore_string_is_null(&val as *const kvstore_string_t) });

    // kvstore_string_free(key);
    unsafe { kvstore_string_free(&mut key as *mut kvstore_string_t) };

    // kvstore_string_free(val);
    unsafe { kvstore_string_free(&mut val as *mut kvstore_string_t) };

    // kvstore_free(store);
    unsafe { kvstore_free(store) };
}
