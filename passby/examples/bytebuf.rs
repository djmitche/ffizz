#![warn(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
#![allow(unused_unsafe)]

use ffizz_passby::Unboxed;

/// ByteBuffer defines a buffer full of bytes.
struct ByteBuffer(Vec<u8>);

/// byte_buffer_t contains a string, for sharing with Rust code.  Its contents are
/// opaque and should not be manipulated.
///
/// ```c
/// strurct byte_buffer_t {
///     _reserved size_t[N];
/// };
/// ```
#[derive(Clone, Copy)]
#[repr(C)]
pub struct byte_buffer_t([u64; 4]); // must be larger than ByteBuffer

type UnboxedByteBuffer = Unboxed<ByteBuffer, byte_buffer_t>;

/// Return a new empty byte_buffer_t.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_new() -> byte_buffer_t {
    unsafe { UnboxedByteBuffer::return_val(ByteBuffer(Vec::new())) }
}

/// Initialize the given byte_buffer_t to an empty value.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_init(bb: *mut byte_buffer_t) {
    unsafe { UnboxedByteBuffer::to_out_param_nonnull(ByteBuffer(Vec::new()), bb) }
}

/// Free a byte_buffer_t.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_free(bb: *mut byte_buffer_t) {
    let bb = unsafe { UnboxedByteBuffer::take_ptr_nonnull(bb) };
    drop(bb); // just to be explicit
}

/// Checksum a byte_buffer_t's contents by XOR'ing all bytes together.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_checksum(bb: *const byte_buffer_t) -> u8 {
    unsafe {
        UnboxedByteBuffer::with_ref_nonnull(bb, |bb| {
            // ok, not the most exciting "checksum"!
            bb.0.iter().copied().reduce(|a, b| a ^ b).unwrap_or(0)
        })
    }
}

/// Add a byte to the byte buffer.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_push(bb: *mut byte_buffer_t, b: u8) {
    unsafe { UnboxedByteBuffer::with_ref_mut_nonnull(bb, |bb| bb.0.push(b)) }
}

/// Combine two byte buffers, returning a new byte buffer containing the bytes
/// from both inputs.  This function consumes its inputs and they must not be
/// used after it returns.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_combine(
    bb1: *mut byte_buffer_t,
    bb2: *mut byte_buffer_t,
) -> byte_buffer_t {
    let mut bb1 = unsafe { UnboxedByteBuffer::take_ptr_nonnull(bb1) };
    let bb2 = unsafe { UnboxedByteBuffer::take_ptr_nonnull(bb2) };

    // modify bb1 in place (but it's not in the caller's location anymore)
    bb1.0.extend(&bb2.0[..]);
    unsafe { UnboxedByteBuffer::return_val(bb1) }
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
