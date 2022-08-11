This crate provides useful traits for implementing a C API to a Rust library.  The traits
provide default implementations that encourage a regular, safe C API.

A typical usage is in a crate providing a C FFI via cbindgen, with a collection of `#[repr(C)]` types and some `#[no_mangle]` functions callable from C.
This style of C API typically has a few "methods" for each type.
For example, a `Node` type in a `palmtree` library might have a few C-accessible fields and C "method" functions like `palmtre_node_new`, `palmtree_node_free`, and `palmtree_node_coconut`.
The functions use methods from this crate to safely transferring values to and from C.

# Safety

This crate doesn't automatically make anything safe.
It just constrains handling of values between C and Rust to a few approaches.
Once these approaches are understood, it's easier for both the Rust and C programmer to understand and abide by the safety requirements.

Each unsafe trait method in this crate describes its safety requirements.
Each call to one of these methods must be in an `unsafe { }` block and that block should be preceded by a comment addressing each safety requirement.
This is illustrated in the examples throughout the documentation.
