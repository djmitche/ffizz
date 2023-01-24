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
    unsafe { ByteBuffer(Vec::new()).to_out_param_nonnull(bb) }
}

/// Free a byte_buffer_t.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_free(bb: *mut byte_buffer_t) {
    let bb = unsafe { ByteBuffer::take_ptr(bb) };
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
    unsafe { ByteBuffer::with_ref_mut(bb, |bb| bb.0.push(b)) }
}

/// Combine two byte buffers, returning a new byte buffer containing the bytes
/// from both inputs.  This function consumes its inputs and they must not be
/// used after it returns.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_combine(
    bb1: *mut byte_buffer_t,
    bb2: *mut byte_buffer_t,
) -> byte_buffer_t {
    let mut bb1 = unsafe { ByteBuffer::take_ptr(bb1) };
    let bb2 = unsafe { ByteBuffer::take_ptr(bb2) };

    // modify bb1 in place (but it's not in the caller's location anymore)
    bb1.0.extend(&bb2.0[..]);
    unsafe { ByteBuffer::return_val(bb1) }
}

fn main() {
    let mut bb1 = unsafe { byte_buffer_new() };
    assert_eq!(
        unsafe { byte_buffer_checksum(&bb1 as *const byte_buffer_t) },
        0
    );

    unsafe {
        byte_buffer_push(&mut bb1 as *mut byte_buffer_t, 0xf0);
        byte_buffer_push(&mut bb1 as *mut byte_buffer_t, 0x0f);
    }

    assert_eq!(
        unsafe { byte_buffer_checksum(&bb1 as *const byte_buffer_t) },
        0xff
    );

    let mut bb2: byte_buffer_t = unsafe { std::mem::zeroed() }; // this is easier in C!
    unsafe {
        byte_buffer_init(&mut bb2 as *mut byte_buffer_t);
    }
    unsafe {
        byte_buffer_push(&mut bb2 as *mut byte_buffer_t, 0xa5);
        byte_buffer_push(&mut bb2 as *mut byte_buffer_t, 0x5b);
    }
    assert_eq!(
        unsafe { byte_buffer_checksum(&bb2 as *const byte_buffer_t) },
        0xfe
    );

    let mut bb3 = unsafe {
        byte_buffer_combine(
            &mut bb1 as *mut byte_buffer_t,
            &mut bb2 as *mut byte_buffer_t,
        )
    };

    // -- note that bb1 and bb2 are invalid now.  Sorry, Rust!

    assert_eq!(
        unsafe { byte_buffer_checksum(&bb3 as *const byte_buffer_t) },
        0xff ^ 0xfe
    );

    unsafe { byte_buffer_free(&mut bb3 as *mut byte_buffer_t) };
}
