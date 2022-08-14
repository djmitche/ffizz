This trait supports values passed to Rust by pointer.
These values are represented as in C, and always handled as pointers.

Typically PassByPointer is used to model objects managed entirely by Rust.
These are represented in the C API by a pointer to an opaque struct, with "new" and "free" functions handling creation and destruction.

# Example

The PassByPointer trait does not require any methods.
See the individual provided methods for examples of their use.

```rust
use ffizz_passby::PassByPointer;
# struct DBEngine { }
# #[allow(non_camel_case_types)]
pub struct foo_db_t (DBEngine);
impl PassByPointer for foo_db_t {}
```
