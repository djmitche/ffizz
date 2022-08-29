This crate provides a string abstraction that is convenient to use from both Rust and C.
It provides a way to pass strings into Rust functions and to return strings to C, with clear rules for ownership.

## Usage

Expose the C type `fz_string_t` in your C header as a struct with the same structure as that in the [`fz_string_t`] docstring.
This is large enough to hold the [`FzString`] type, and ensures the C compiler will properly align the value.

You may call the type whatever you like.
Type names are erased in the C ABI, so it's fine to write a Rust declaration using `fz_string_t` and equivalent C declaration using `your_name_here_t`.
You may also rename the Rust type with `use ffizz_string::fz_string_t as ..`, if you prefer.

### As an Argument

Define your `extern "C"` function to take a `*mut fz_string_t` argument:

```ignore
pub unsafe extern "C" fn is_a_color_name(name: *mut fz_string_t) -> bool { .. };
```

Then use one of the FzString methods to handle the string value.
As standard practice, address each of the items listed in the "Safety" section of each unsafe method you call.
For example:

```ignore
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

### As a Return Value

To return a string, define your `extern "C"` function to return an `fz_string_t`:
```ignore
pub unsafe extern "C" fn favorite_color() -> fz_string_t { .. }
```

Then use `FzString::return_val` to return the value:
```ignore
// SAFETY:
//  - caller will free the returned string (see docstring)
unsafe {
    return FzString::return_val(color);
}
```

## Example

See the `kv` example in this crate for a worked example of a simple library using `ffizz_string`.

## Performance

The implementation is general-purpose, and may result in more allocations or string copies than strictly necessary.
This is particularly true if the Rust implementation immediately converts `FzString` into `std::string::String`.
This conversion brings great simplicity, but involves an allocation and a copy of the string.

In situations where API performance is critical, it may be preferable to create a purpose-specific string implementation, perhaps inspired by this crate.
