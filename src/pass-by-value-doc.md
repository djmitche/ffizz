This trait supports passing data to Rust by value.

Pass-by-values implies that values are copyable, via assignment in C, so this
trait is typically used to represent Copy values, and in particular values that
do not contain pointers.

The Rust and C types may differ, with [`PassByValue::from_ctype`] and [`PassByValue::as_ctype`]
converting between them.  Implement this trait for the C type and specify the
Rust type as [`PassByValue::RustType`].

The RustType must be droppable (not containing raw pointers).

# Examples

In most cases, this trait is used for simple types like enums with values.

```rust
# use uuid::Uuid;
# use ffi_passby::PassByValue;

#[repr(C)]
pub struct foo_status_t {
    status: u8,
    errno: u32,
};

pub const FOO_STATUS_READY: u8 = 1;
pub const FOO_STATUS_RUNNING: u8 = 2;
pub const FOO_STATUS_FAILED: u8 = 3;

pub enum Status {
    Ready,
    Running,
    Failed(u32),
}

impl PassByValue for foo_status_t {
    type RustType = Status;

    unsafe fn from_ctype(self) -> Self::RustType {
        match self.status {
            FOO_STATUS_READY => Status::Ready,
            FOO_STATUS_RUNNING => Status::Running,
            FOO_STATUS_FAILED => Status::Failed(self.errno),
            _ => panic!("invalid status value"),
        }
    }

    fn as_ctype(arg: Self::RustType) -> Self {
        match arg {
            Status::Ready => foo_status_t {
                status: FOO_STATUS_READY,
                errno: 0,
            },
            Status::Running => foo_status_t {
                status:FOO_STATUS_RUNNING,
                errno: 0,
            },
            Status::Failed(errno) => foo_status_t {
                status:FOO_STATUS_FAILED,
                errno: errno,
            },
        }
    }
}

/// Get the current system status.
#[no_mangle]
pub unsafe extern "C" fn foo_system_status() -> foo_status_t {
    let status: Status = Status::Ready;
    // SAFETY:
    // - status is not allocated
    unsafe { foo_status_t::return_val(status) }
}
```

The trait can also be used for C representations of more interesting data types:

```rust
# use uuid::Uuid;
# use ffi_passby::PassByValue;

/// foo_uuid_t stores a UUID.
///
/// cbindgen:field-names=[bytes]
#[repr(C)]
pub struct foo_uuid_t([u8; 16]);

impl PassByValue for foo_uuid_t {
    type RustType = Uuid;

    unsafe fn from_ctype(self) -> Self::RustType {
        Uuid::from_bytes(self.0)
    }

    fn as_ctype(arg: Uuid) -> Self {
        foo_uuid_t(*arg.as_bytes())
    }
}

