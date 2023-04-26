This crate provides a string abstraction that is convenient to use from both Rust and C.
It provides a way to pass strings into Rust functions and to return strings to C, with clear rules for ownership.

## Usage

The types in this crate are specializations of a `ffizz_passby::OpaqueStruct`.
See the documentation `ffizz-passby` crate for more general guidance on creating effective C APIs.

### String Type

Expose the C type `fz_string_t` in your C header as a struct with the same structure as that in the [`fz_string_t`] docstring.
This is large enough to hold the [`FzString`] type, and ensures the C compiler will properly align the value.

You may call the type whatever you like.
Type names are erased in the C ABI, so it's fine to write a Rust declaration using `fz_string_t` and equivalent C declaration using `mystrtype_t`.
You may also rename the Rust type with `use ffizz_string::fz_string_t as ..`, if you prefer.

### String Utility Functions

This crate includes a number of utility functions, named `fz_string_..`.
These can be re-exported to C using whatever names you prefer, and with docstrings based on those in this crate, including C declarations:

```ignore
ffizz_snippet!{
#[ffizz(name="mystrtype_free")]
/// Free a mystrtype_t.
///
/// # Safety
///
/// The string must not be used after this function returns, and must not be freed more than once.
/// It is safe to free Null-variant strings.
///
/// ```c
/// EXTERN_C void mystrtype_free(mystrtype_t *);
/// ```
}
ffizz_string::reexport!(fz_string_free as mystrtype_free);
```

### Strings as Function Arguments

There are two design decisions to make when accepting strings as function arguments.
First, does ownership of the string transfer from the caller to the callee?
Or in Rust terms, is the value moved?
This is largely a matter of convenience for the callers, but it's best to be consistent throughout an API.

Second, do you want to pass strings by value or pointer?
Passing by pointer is recommended as it is typically more efficient and allows invalidating moved values in a way that prevents use-after-free errors.

#### By Pointer

Define your `extern "C"` function to take a `*mut fz_string_t` argument:

```ignore
pub unsafe extern "C" fn is_a_color_name(name: *const fz_string_t) -> bool { .. };
```

If taking ownership of the value, use [`FzString::take_ptr`].
Otherwise, use [`FzString::with_ref`] or [`FzString::with_ref_mut`] to borrow a reference from the pointer.

All of these methods are unsafe.
As standard practice, address each of the items listed in the "Safety" section of each unsafe method you call.
These can often reference the docstring appearing in the C header, as it is generally the responsibilty of the C caller to ensure these requirements are met.
For example:

```ignore
ffizz_snippet!{
#[ffizz(name="mystrtype_free")]
/// Determine whether the given string contains a color name.
///
/// # Safety
///
/// The name argument must not be NULL.
///
/// ```c
/// EXTERN_C bool is_a_color_name(const fz_string_t *);
/// ```
}
pub unsafe extern "C" fn is_a_color_name(name: *const fz_string_t) -> bool { .. };
// SAFETY:
//  - name is not NULL (see docstring)
//  - no other thread will mutate name (type is documented as not threadsafe)
unsafe {
    FzString::with_ref(name, |name| {
        if let Some(name) = name.as_str() {
            return Colors::from_str(name).is_some();
        }
        false // invalid UTF-8 is _not_ a color name
    })
}
```

#### By Value

Alternatively, you may require callers to pass the string by value.
Declare your functions like this:

```ignore
pub unsafe extern "C" fn is_a_color_name(name: fz_string_t) -> bool { .. };
```

Then, use [`FzString::take`] to take ownership of the string as a Rust value.
There is no option for the caller to retain ownership when passing by value.

#### Always Take Everything

If your C API definition indicates that a function takes ownership of values in its function arguments, take ownersihp of _all_ arguments before any early returns can occur.
For example:

```ignore
pub unsafe extern "C" convolve_strings(a: *const fz_string_t, b: *const fz_string_t) -> bool {
    // SAFETY: ...
    let a = unsafe { FzString::take_ptr(a) };
    if a.len() == 0 {
        return false; // BUG
    }
    // SAFETY: ...
    let b = unsafe { FzString::take_ptr(b) }; // BAD!
    // ...
}
```

Here, if `a` is invalid, the function will not free `b`, despite the API contract promising to do so.
To fix, move the `let b` statement before the early return.

### Strings as Return Values

To return a string, define your `extern "C"` function to return an `fz_string_t`:
```ignore
pub unsafe extern "C" fn favorite_color() -> fz_string_t { .. }
```

Then use [`FzString::return_val`] to return the value:
```ignore
pub unsafe extern "C" fn favorite_color() -> fz_string_t {
    let color = FzString::from("raw umber");
    // SAFETY:
    //  - caller will free the returned string (see docstring)
    unsafe {
        return FzString::return_val(color);
    }
}
```

### Strings as Out Parameters

An "out parameter" is a common idiom in C and C++.
To return a string into an out parameter, use [`FzString::to_out_param`] or [`FzString::to_out_param_nonnull`]:

```ignore

/// Determine the complement of the given color, returning true on success. If
/// the color cannot be complemented, return false and leave the
/// `complement_out` string uninitialized.
pub unsafe extern "C" fn complement_color(
    color: *const fz_string_t,
    complement_out: *mut fz_string_t) -> fz_string_t {
    result = FzString::from("opposite");
    unsafe {
        FzString::to_out_param(complement_out, result);
    }
    true
}
```

## Example

See [the `kv` example](https://github.com/djmitche/ffizz/blob/main/string/examples/kv.rs) in this crate for a worked example of a simple library using `ffizz_string`.

## Performance

The implementation is general-purpose, and may result in more allocations or string copies than strictly necessary.
This is particularly true if the Rust implementation immediately converts `FzString` into `std::string::String`.
This conversion brings great simplicity, but involves an allocation and a copy of the string.

In situations where API performance is critical, it may be preferable to use `FzString` throughout the implementation.
