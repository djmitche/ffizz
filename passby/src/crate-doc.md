This crate provides useful traits for implementing a C API to a Rust library.  The traits
provide default implementations that encourage a regular, safe C API.

A typical usage is in a crate providing a C API, with a collection of `#[repr(C)]` types and some `#[no_mangle]` functions callable from C.
This style of C API typically has a few "methods" for each type.
For example, a `Node` type in a `palmtree` library might have a few C-accessible fields and C "method" functions like `palmtree_node_new`, `palmtree_node_free`, and `palmtree_node_coconut`.
The functions use methods from this crate to safely transferring values to and from C.

# Safety

This crate doesn't automatically make anything safe.
It just constrains handling of values between C and Rust to a few approaches.
Once these approaches are understood, it's easier for both the Rust and C programmer to understand and abide by the safety requirements.

Each unsafe trait method in this crate describes its safety requirements.
Each call to one of these methods must be in an `unsafe { }` block and that block should be preceded by a comment addressing each safety requirement.
This is illustrated in the examples throughout the documentation.

## General Advice

In general, C APIs rely on programmers to carefully read the API documentation and follow its rules, without help from the compiler.
Where possible, design APIs to make this easy, with simple-to-remember rules and runtime checks where practical.

### Taking Ownership

When a function is documented as taking ownership of a value, ensure that this is always the case, even if an error occurs.
For example, consider an infinite-precision division function:

```ignore
unsafe extern "C" fn infprec_div(a: infprec_t, b: infprec_t, c_out: *infprec_t) -> bool {
    let b = InfinitePrecision::take(b);
    if b.is_zero() {
        return false;
    }
    let a = InfinitePrecision::take(a);
    InfinitePrecision::return_value(c_out, a / b);
    true
}
```

The caller expects that `infprec_div` takes ownership of `a` and `b`, and will not free them on return.
As written, when `b` is zero, the early return occurs before `a` has been converted to a Rust value, so it will not be dropped, and will leak.
The fix, in this case, is to move the `let a` statement before the early return.

### Hidden Mutability

Rust makes a strict distinction between a shared, read-only reference and an exclusive, mutable reference.
C typically makes no such distinction, and avoids data races though careful comments as to which methods can be called concurrently.
In most cases, C programmers will "guess" what methods can be called concurrently.
A good C API will make these guesses explicit.

For example, a type might be documented as not threadsafe, where no functions may be called concurrently for a single value of the type.
For many types, however, read-only methds can be called concurrently, as long as they are not concurrent with modifications.
For example, it may be safe to call `kvstore_get` on a `kvstore_t` concurrently, as long as those calls do not overlap ay `kvstore_set` calls.

Interior mutability requires even more careful documentation.
Continuing the example, perhaps the `kvstore_t` data structure rebalances itself on read, in which case `kvstore_get` is _not_ safe to call concurrently, even though it appears to be a read-only operation.
