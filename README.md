# ffizz

*ffizz* is a library of utilities for exporting Rust libs for use in other languages.

FFI generally requires a lot of unsafe code, which in turn requires a lot of manual verification of assumptions.
The ffizz libraries help by implementing some common patterns with clear safety guidelines that are easily described in the documentation for the C side of the interface.

Ffizz differs from other FFI utilities like [uniffi](https://github.com/mozilla/uniffi-rs), [cxx](https://cxx.rs/), or [diplomat](https://rust-diplomat.github.io/book/) in that it does not generate bindings.
Instead, it's aimed at situations where you want precise control over the interface, in a safe and ergonomic fashion.
If you want to implement exactly the C API you have in mind, Ffizz is the tool.

## Crates

* [ffizz-passby](https://docs.rs/ffizz-passby) supports passing arguments and return values by pointer or by value.
* [ffizz-header](https://docs.rs/ffizz-header) supports generating a C header corresponding to a library crate
* [ffizz-string](https://docs.rs/ffizz-string) provides a simple string abstraction
