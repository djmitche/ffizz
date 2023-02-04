This crate provides useful types for implementing a C API to a Rust library.
The structs provide utility functions that encourage a regular, safe C API.

A typical usage is in a crate providing a C API, with a collection of `#[repr(C)]` types and some `#[no_mangle]` functions callable from C.
This style of C API typically has a few "methods" for each type.
For example, a `Node` type in a `palmtree` library might have a few C-accessible fields and C "method" functions like `palmtree_node_new`, `palmtree_node_free`, and `palmtree_node_coconut`.
The functions use methods from this crate to safely transfer values to and from C.

# Usage

Each of the provided types is an empty struct exposing utility methods.
To use these types, define a type alias specifying the relevant Rust and (if necessary) C types:

```
# struct InfPrec(u32);
# #[repr(C)]
# struct infprec_t(u32);
# impl From<InfPrec> for infprec_t {
#     fn from(rval: InfPrec) -> infprec_t { infprec_t(rval.0) }
# }
# impl Into<InfPrec> for infprec_t {
#     fn into(self) -> InfPrec { InfPrec(self.0) }
# }
type InfPrecValue = ffizz_passby::Value<InfPrec, infprec_t>;
```

Then, use utility functions on that type as necessary:

```
# struct InfPrec(u32);
# #[repr(C)]
# struct infprec_t(u32);
# impl From<InfPrec> for infprec_t {
#     fn from(rval: InfPrec) -> infprec_t { infprec_t(rval.0) }
# }
# impl Into<InfPrec> for infprec_t {
#     fn into(self) -> InfPrec { InfPrec(self.0) }
# }
# type InfPrecValue = ffizz_passby::Value<InfPrec, infprec_t>;
let unlucky = InfPrec(13);
// SAFETY: value does not contain an allocation
unsafe {
    let cval = InfPrecValue::return_val(unlucky);
}
```

The following types are available:

 * [`Value`], which allows passing simple `Copy`-able values to and from C.
 * [`Boxed`], which allows passing value by pointer, where Rust to manages the allocation.
 * [`Unboxed`], which allows passing a value by pointer, but where C manages the allocation (such as on the stack or in some other struct).

# Safety

This crate doesn't automatically make anything safe.
It just constrains handling of values between C and Rust to a few approaches.
Once these approaches are understood, it's easier for both the Rust and C programmer to understand and abide by the safety requirements.

Each unsafe trait method in this crate describes its safety requirements.
Each call to one of these methods must be in an `unsafe { }` block and that block should be preceded by a comment addressing each safety requirement.
This is illustrated in the examples throughout the documentation.

## General Advice

In general, C APIs rely on programmers to carefully read the API documentation and follow its rules, without help from the compiler.
Where possible, design APIs to make this easy, with simple-to-remember rules, consistent behavior, and runtime checks where practical.

## Taking Ownership

When a function is documented as taking ownership of a value, ensure that this is always the case, even if an error occurs.
For example, consider an infinite-precision division function:

```
# struct InfPrec(u32);
# impl InfPrec {
#     fn is_zero(&self) -> bool { self.0 == 0 }
# }
# impl std::ops::Div<InfPrec> for InfPrec {
#     type Output = Self;
#     fn div(self, other: Self) -> Self { Self(self.0 / other.0) }
# }
# #[repr(C)]
# struct infprec_t(u32);
type InfinitePrecision = ffizz_passby::Unboxed<InfPrec, infprec_t>;
unsafe extern "C" fn infprec_div(a: infprec_t, b: infprec_t, c_out: *mut infprec_t) -> bool {
    // SAFETY: b is valid and caller will not use it again (documented in API)
    let b = unsafe { InfinitePrecision::take(b) };
    if b.is_zero() {
        return false;
    }
    // SAFETY: a is valid and caller will not use it again (documented in API)
    let a = unsafe { InfinitePrecision::take(a) };
    // SAFETY:
    //  - c_out is not NULL, properly aligned, and has enough space for an infprec_t (documented in API)
    //  - c_out will eventually be passed back to Rust to be freed.
    unsafe {
        InfinitePrecision::to_out_param_nonnull(a / b, c_out);
    }
    true
}
```

The caller expects that `infprec_div` takes ownership of `a` and `b`, and will not free them on return.
As written, when `b` is zero, the early return occurs before `a` has been converted to a Rust value, so it will not be dropped, and will leak.
The fix, in this case, is to move the `let a` statement before the early return.

## Hidden Mutability

Rust makes a strict distinction between a shared, read-only reference and an exclusive, mutable reference.
C typically makes no such distinction, and avoids data races though careful comments as to which methods can be called concurrently.
In most cases, C programmers will "guess" what methods can be called concurrently.
A good C API will make these guesses explicit.

For example, a type might be documented as not threadsafe, where no functions may be called concurrently for a single value of the type.
For many types, however, read-only methds can be called concurrently, as long as they are not concurrent with modifications.
For example, it may be safe to call `kvstore_get` on a `kvstore_t` concurrently, as long as those calls do not overlap any `kvstore_set` calls.

Interior mutability requires even more careful documentation.
Continuing the example, perhaps the `kvstore_t` data structure rebalances itself on read, in which case `kvstore_get` is _not_ safe to call concurrently, even though it appears to be a read-only operation.
In Rust, the signature would be `KVStore::get(&mut self)` and the compiler would prevent such concurrent calls.
In C, this must be explained clearly in the documentation.
