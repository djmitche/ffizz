This trait supports passing data to Rust by value.  These are represented as
full structs in C.  Such values are implicitly copyable, via C's struct
assignment.

The Rust and C types may differ, with [`PassByValue::from_ctype`] and [`PassByValue::as_ctype`]
converting between them.  Implement this trait for the C type and specify the
Rust type as [`PassByValue::RustType`].

The RustType must be droppable (not containing raw pointers).

# Example

```rust
use uuid::Uuid;
use ffi_passby::PassByValue;

/// CUuid is used as a task identifier.  Uuids do not contain any pointers and need not be freed.
/// Uuids are typically treated as opaque, but the bytes are available in big-endian format.
///
/// cbindgen:field-names=[bytes]
#[repr(C)]
pub struct CUuid([u8; 16]);

impl PassByValue for CUuid {
    type RustType = Uuid;

    unsafe fn from_ctype(self) -> Self::RustType {
        // SAFETY:
        //  - any 16-byte value is a valid Uuid
        Uuid::from_bytes(self.0)
    }

    fn as_ctype(arg: Uuid) -> Self {
        CUuid(*arg.as_bytes())
    }
}

/// Create a new, randomly-generated UUID.
#[no_mangle]
pub unsafe extern "C" fn make_uuid() -> CUuid {
    // SAFETY:
    // - value is not allocated
    unsafe { CUuid::return_val(Uuid::new_v4()) }
}

/// Determine the version for the given UUID.
#[no_mangle]
pub unsafe extern "C" fn uuid_version(cuuid: CUuid) -> usize {
    // SAFETY:
    // - cuuid is a valid CUuid (all bytes are valid)
    // - cuuid is Copy so ownership doesn't matter
    let uuid = unsafe { CUuid::val_from_arg(cuuid) };
    return uuid.get_version_num()
}
```
