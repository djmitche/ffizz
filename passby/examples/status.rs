#![warn(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use ffizz_passby::{PassByPointer, PassByValue};

/// A simple little state machine for a system's status.  This module
/// represents the Rust library being exposed via FFI.
mod hittr {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Status {
        Ready,
        Running { count: u32 },
        Failed,
    }

    pub struct System {
        pub status: Status,
    }

    impl System {
        pub fn new() -> System {
            System {
                status: Status::Ready,
            }
        }

        pub fn new_network(_port: u16) -> Result<System, ()> {
            // (this constructor is just to have an example of a fallible constructor)
            Ok(System {
                status: Status::Ready,
            })
        }

        pub fn run(&mut self) {
            if self.status != Status::Ready {
                self.status = Status::Failed;
            } else {
                self.status = Status::Running { count: 0 };
            }
        }

        pub fn count_hit(&mut self) {
            if let Status::Running { count } = self.status {
                if count >= 5 {
                    self.status = Status::Failed;
                    return;
                }
                self.status = Status::Running { count: count + 1 };
            } else {
                self.status = Status::Failed;
            }
        }
    }
}

mod status {
    use super::hittr::Status;
    use ffizz_passby::PassByValue;

    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct hittr_status_t {
        pub status: u8,
        pub count: u32,
    }

    pub const HITTR_STATUS_READY: u8 = 1;
    pub const HITTR_STATUS_RUNNING: u8 = 2;
    pub const HITTR_STATUS_FAILED: u8 = 3;

    impl PassByValue for hittr_status_t {
        type RustType = Status;

        unsafe fn from_ctype(self) -> Self::RustType {
            match self.status {
                HITTR_STATUS_READY => Status::Ready,
                HITTR_STATUS_RUNNING => Status::Running { count: self.count },
                HITTR_STATUS_FAILED => Status::Failed,
                _ => panic!("invalid status value"),
            }
        }

        fn as_ctype(arg: Self::RustType) -> Self {
            match arg {
                Status::Ready => hittr_status_t {
                    status: HITTR_STATUS_READY,
                    count: 0,
                },
                Status::Running { count } => hittr_status_t {
                    status: HITTR_STATUS_RUNNING,
                    count,
                },
                Status::Failed => hittr_status_t {
                    status: HITTR_STATUS_FAILED,
                    count: 0,
                },
            }
        }
    }
}

mod system {
    use super::hittr::System;
    use ffizz_passby::PassByPointer;

    /// This opaque struct represents a running Hittr system.
    ///
    /// Values of this type are not threadsafe and must be accessed by only
    /// one thread at a time.
    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct hittr_system_t(pub System);

    impl PassByPointer for hittr_system_t {}
}

use hittr::*;
use status::*;
use system::*;

/// Create a new Hittr system.
///
/// # Safety
///
/// The returned hittr_system_t must be freed with hittr_system_free.
#[no_mangle]
pub unsafe extern "C" fn hittr_system_new() -> *mut hittr_system_t {
    let sys = System::new();
    // SAFETY: function docs indicate value must be freed
    unsafe { hittr_system_t(sys).return_ptr() }
}

/// Create a new Hittr system with a network port.  This returns true
/// on success.  On failure, the output argument is not changed.
///
/// # Safety
///
/// The system_out argument must ne non-NULL and point to a valid, properly aligned
/// `*hittr_system_t`.  The returned hittr_system_t must be freed with hittr_system_free.
#[no_mangle]
pub unsafe extern "C" fn hittr_system_new_network(
    system_out: *mut *mut hittr_system_t,
    port: u16,
) -> bool {
    if let Ok(sys) = System::new_network(port) {
        // SAFETY: see docstring
        unsafe { hittr_system_t(sys).ptr_to_arg_out(system_out) }
        true
    } else {
        false
    }
}

/// Free a Hittr system.
///
/// # Safety
///
/// The system must be non-NULL and point to a valid hittr_system_t. After this call it is no
/// longer valid and must not be used.
#[no_mangle]
pub unsafe extern "C" fn hittr_system_free(system: *mut hittr_system_t) {
    // SAFETY:
    //  - system is valid and not NULL (see docstring)
    //  - caller will not use system after this call (see docstring)
    let system = unsafe { hittr_system_t::take_from_ptr_arg(system) };
    drop(system); // (Rust would do this anyway, but let's be explicit)
}

/// Run the Hittr system.
///
/// If the sytem is already running, it will enter the failed state.
///
/// # Safety
///
/// The system must be non-NULL and point to a valid hittr_system_t.
#[no_mangle]
pub unsafe extern "C" fn hittr_system_run(system: *mut hittr_system_t) {
    // SAFETY:
    // - system is not NULL and valid (see docstring)
    // - system is valid for the life of this function (documented as not threadsafe)
    // - system will not be accessed during the life of this function (documented as not threadsafe)
    let system = &mut unsafe { hittr_system_t::from_ptr_arg_ref_mut(system) }.0;
    system.run();
}

/// Record a hit on thi Hittr system.
///
/// If the sytem is not running, it will enter the failed state.  If it counts 5
/// or more hits, it will enter the failed.state.
///
/// # Safety
///
/// The system must be non-NULL and point to a valid hittr_system_t.
#[no_mangle]
pub unsafe extern "C" fn hittr_system_count_hit(system: *mut hittr_system_t) {
    // SAFETY:
    // - system is not NULL and valid (see docstring)
    // - system is valid for the life of this function (documented as not threadsafe)
    // - system will not be accessed during the life of this function (documented as not threadsafe)
    let system = &mut unsafe { hittr_system_t::from_ptr_arg_ref_mut(system) }.0;
    system.count_hit();
}

/// Get the current system status.
///
/// The system must be non-NULL and point to a valid hittr_system_t.
#[no_mangle]
pub unsafe extern "C" fn hittr_system_status(system: *const hittr_system_t) -> hittr_status_t {
    // SAFETY:
    // - system is not NULL and valid (see docstring)
    // - system is valid for the life of this function (documented as not threadsafe)
    // - system will not be modified during the life of this function (documented as not threadsafe)
    let system = &unsafe { hittr_system_t::from_ptr_arg_ref(system) }.0;
    // SAFETY:
    // - hittr_status_t is not allocated, so no issues
    unsafe { hittr_status_t::return_val(system.status) }
}

fn main() {
    let sys = unsafe { hittr_system_new() };

    let st = unsafe { hittr_system_status(sys) };
    assert_eq!(st.status, HITTR_STATUS_READY);
    assert_eq!(st.count, 0);

    unsafe { hittr_system_run(sys) };

    let st = unsafe { hittr_system_status(sys) };
    assert_eq!(st.status, HITTR_STATUS_RUNNING);
    assert_eq!(st.count, 0);

    for i in 1..=5 {
        unsafe { hittr_system_count_hit(sys) };
        let st = unsafe { hittr_system_status(sys) };
        assert_eq!(st.status, HITTR_STATUS_RUNNING);
        assert_eq!(st.count, i);
    }

    unsafe { hittr_system_count_hit(sys) }; // 5th hit causes system failure
    let st = unsafe { hittr_system_status(sys) };
    assert_eq!(st.status, HITTR_STATUS_FAILED);
    assert_eq!(st.count, 0);

    unsafe { hittr_system_free(sys) };

    // this is awkward to call from Rust, but would be pretty natural in C
    let mut sys: *mut hittr_system_t = std::ptr::null_mut();
    assert!(unsafe { hittr_system_new_network(&mut sys as *mut *mut hittr_system_t, 1300) });
    let st = unsafe { hittr_system_status(sys) };
    assert_eq!(st.status, HITTR_STATUS_READY);
    assert_eq!(st.count, 0);
}
