This crate supports generating a C header for a library crate, directly in the library itself.

Typically the crate would be annotated with `ffizz_header` macros to define the header content.
Then, the header is generated by calling [`generate`].

# Generating Headers

What follows is a simple, effective way to generate the header file, using the excellent [cargo-xtask](https://github.com/matklad/cargo-xtask/).
With this in place, simply run `cargo xtask codegen` to generate the header file.
The file can either be checked in (in which case CI should verify that it is up-to-date), or generated as part of the release / packaging process.

## Setup

In your library's top level, add a call to 

```ignore
#[cfg(debug_assertions)] // only include this in debug builds
/// Generate the header
pub fn generate_header() -> String {
    ffizz_header::generate()
}
```

Set up an xtask project as described in [the project's documentation](https://github.com/matklad/cargo-xtask/).
Add your library as a dependency of the xtask crate.
In `xtask/src/main.rs`, for a `mysupercool-lib` crate:

```ignore
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_dir = manifest_dir.parent().unwrap();

    // assume the mysupercool-lib crate is in `lib/`..
    let lib_crate_dir = workspace_dir.join("tests").join("lib");
    let mut file = File::create(lib_crate_dir.join("mysupercoollib.h")).unwrap();
    write!(&mut file, "{}", ::mysupercool_lib::generate_header()).unwrap();
}
```

You may wish to improve on this implementation, with proper command-line parsing and error handling.

### Caveats

This method does not support producing multiple header files for a single workspace.
Rust refuses to link them, due to duplicate symbols.
If your workspace contains multiple libraries, another option is to build a binary for each one, that generates the header file for only that library.

## Defining Headers

Typically, a library exporting a header will define its topmatter and corresponding footer in `src/lib.rs`, using [`snippet`].

```
ffizz_header::snippet! {
#[ffizz(name="topmatter", order=1)]
/// ```c
/// #ifndef INFPREC_H
/// #define INFPREC_H
///
/// #include <stdint.h> // ..and any other required headesr
/// ```
}

ffizz_header::snippet! {
#[ffizz(name="bottomatter", order=10000)]
/// ```c
/// #endif /* INFPREC_H */
/// ```
}
```

The topmatter might also include forward declarations of types or macros.

The remaining declarations will be for types and exported functions, using [`item`].
It can be helpful to define a range of `order` values for each source file, to keep related declarations together in the generated header.

```
#[ffizz_header::item]
#[ffizz(order = 900)]
/// ***** infprec_t *****
///
/// An infinite-precision integer.
/// ```c
/// typedef struct infprec_t infprec_t
/// ```
pub struct InfPrec { /* .. */ }
```

```
# type infprec_t = ();
#[ffizz_header::item]
#[ffizz(order = 901)]
/// Add two infinite-precision numbers.
#[no_mangle]
pub unsafe extern "C" fn infprec_add(a: infprec_t,  b: infprec_t) -> infprec_t { todo!() }
```

### `extern "C"`

For headers intended for use in C and C++, it may be helpful to define an EXTERN_C macro:

```c
/// #ifdef __cplusplus
/// #define EXTERN_C extern "C"
/// #else
/// #define EXTERN_C
/// #endif // __cplusplus
```

this can later be used in declarations like
```c
EXTERN_C infprec_t infprec_add(infprec_t a, infprec_t b);
```
