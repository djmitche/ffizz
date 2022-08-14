#![warn(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use ffizz_passby::OpaqueStruct;

/// ByteBuffer defines a buffer full of bytes.
struct ByteBuffer(Vec<u8>);

/// byte_buffer_t contains a string, for sharing with Rust code.  Its contents are
/// opaque and should not be manipulated.
///
/// cbindgen:field-names=[_reserved]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct byte_buffer_t([u64; 4]); // must be larger than ByteBuffer

impl OpaqueStruct for ByteBuffer {
    type CType = byte_buffer_t;
}

/// Return a new empty byte_buffer_t.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_new() -> byte_buffer_t {
    unsafe { ByteBuffer::return_val(ByteBuffer(Vec::new())) }
}

/// Initialize the given byte_buffer_t to an empty value.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_init(bb: *mut byte_buffer_t) {
    unsafe { ByteBuffer::initialize(bb, ByteBuffer(Vec::new())) }
}

/// Free a byte_buffer_t.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_free(bb: *mut byte_buffer_t) {
    let bb = unsafe { ByteBuffer::take(bb) };
    drop(bb); // just to be explicit
}

/// Checksum a byte_buffer_t's contents by XOR'ing all bytes together.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_checksum(bb: *const byte_buffer_t) -> u8 {
    unsafe {
        ByteBuffer::with_ref(bb, |bb| {
            // ok, not the most exciting "checksum"!
            bb.0.iter().copied().reduce(|a, b| a ^ b).unwrap_or(0)
        })
    }
}

/// Add a byte to the byte buffer.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_push(bb: *mut byte_buffer_t, b: u8) {
    unsafe { ByteBuffer::with_mut_ref(bb, |bb| bb.0.push(b)) }
}

fn main() {
    let mut bb = unsafe { byte_buffer_new() };
    assert_eq!(
        unsafe { byte_buffer_checksum(&bb as *const byte_buffer_t) },
        0
    );

    unsafe {
        byte_buffer_push(&mut bb as *mut byte_buffer_t, 0xf0);
        byte_buffer_push(&mut bb as *mut byte_buffer_t, 0x0f);
    }

    assert_eq!(
        unsafe { byte_buffer_checksum(&bb as *const byte_buffer_t) },
        0xff
    );

    unsafe { byte_buffer_free(&mut bb as *mut byte_buffer_t) };

    // note: `bb` is uninitialized here -- testing C APIs in Rust is hard!

    unsafe {
        byte_buffer_init(&mut bb as *mut byte_buffer_t);
    }
    assert_eq!(
        unsafe { byte_buffer_checksum(&bb as *const byte_buffer_t) },
        0
    );
    unsafe { byte_buffer_free(&mut bb as *mut byte_buffer_t) };
}
