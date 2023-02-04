#![warn(unsafe_op_in_unsafe_fn)]
#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::new_without_default)]

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

    /// ```c
    /// typedef struct hittr_system_t hittr_system_t;
    /// ```
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
    use ffizz_passby::Value;

    #[allow(non_camel_case_types)]
    #[repr(C)]
    pub struct hittr_status_t {
        pub status: u8,
        pub count: u32,
    }

    pub const HITTR_STATUS_READY: u8 = 1;
    pub const HITTR_STATUS_RUNNING: u8 = 2;
    pub const HITTR_STATUS_FAILED: u8 = 3;

    impl Into<Status> for hittr_status_t {
        fn into(self) -> Status {
            match self.status {
                HITTR_STATUS_READY => Status::Ready,
                HITTR_STATUS_RUNNING => Status::Running { count: self.count },
                HITTR_STATUS_FAILED => Status::Failed,
                _ => panic!("invalid status value"),
            }
        }
    }

    impl From<Status> for hittr_status_t {
        fn from(rval: Status) -> hittr_status_t {
            match rval {
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

    pub type StatusValue = Value<Status, hittr_status_t>;
}

use ffizz_passby::Boxed;
use hittr::*;
use status::*;

type BoxedSystem = Boxed<System>;

/// Create a new Hittr system.
///
/// # Safety
///
/// The returned hittr_system_t must be freed with hittr_system_free.
///
/// ```c
/// hittr_system_t *hittr_system_new();
/// ```
#[no_mangle]
pub unsafe extern "C" fn hittr_system_new() -> *mut System {
    let sys = System::new();
    // SAFETY: function docs indicate value must be freed
    unsafe { BoxedSystem::return_val(sys) }
}

/// Create a new Hittr system with a network port.  This returns true
/// on success.  On failure, the output argument is not changed.
///
/// # Safety
///
/// The system_out argument must ne non-NULL and point to a valid, properly aligned
/// `*hittr_system_t`.  The returned hittr_system_t must be freed with hittr_system_free.
///
/// ```c
/// bool hittr_system_new_network(hittr_system_t **system_out, uint16_t port);
/// ```
#[no_mangle]
pub unsafe extern "C" fn hittr_system_new_network(system_out: *mut *mut System, port: u16) -> bool {
    if let Ok(sys) = System::new_network(port) {
        // SAFETY: see docstring
        unsafe { BoxedSystem::to_out_param(sys, system_out) }
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
///
/// ```c
/// void hittr_system_free(hittr_system_t *system);
/// ```
#[no_mangle]
pub unsafe extern "C" fn hittr_system_free(system: *mut System) {
    // SAFETY:
    //  - system is valid and not NULL (see docstring)
    //  - caller will not use system after this call (see docstring)
    unsafe { BoxedSystem::take_nonnull(system) };
    // (System is implicitly dropped)
}

/// Run the Hittr system.
///
/// If the sytem is already running, it will enter the failed state.
///
/// # Safety
///
/// The system must be non-NULL and point to a valid hittr_system_t.
///
/// ```c
/// void hittr_system_run(hittr_system_t *system);
/// ```
#[no_mangle]
pub unsafe extern "C" fn hittr_system_run(system: *mut System) {
    // SAFETY:
    // - system is not NULL and valid (see docstring)
    // - system is valid for the life of this function (documented as not threadsafe)
    // - system will not be accessed during the life of this function (documented as not threadsafe)
    unsafe {
        BoxedSystem::with_ref_mut_nonnull(system, |system| {
            system.run();
        });
    }
}

/// Record a hit on thi Hittr system.
///
/// If the sytem is not running, it will enter the failed state.  If it counts 5
/// or more hits, it will enter the failed.state.
///
/// # Safety
///
/// The system must be non-NULL and point to a valid hittr_system_t.
///
/// ```c
/// void hittr_system_count_hit(hittr_system_t *system);
/// ```
#[no_mangle]
pub unsafe extern "C" fn hittr_system_count_hit(system: *mut System) {
    // SAFETY:
    // - system is not NULL and valid (see docstring)
    // - system is valid for the life of this function (documented as not threadsafe)
    // - system will not be accessed during the life of this function (documented as not threadsafe)
    unsafe {
        BoxedSystem::with_ref_mut_nonnull(system, |system| {
            system.count_hit();
        });
    }
}

/// Get the current system status.
///
/// The system must be non-NULL and point to a valid hittr_system_t.
///
/// ```c
/// hittr_status_t hittr_system_status(hittr_system_t *system);
/// ```
#[no_mangle]
pub unsafe extern "C" fn hittr_system_status(system: *const System) -> hittr_status_t {
    // SAFETY:
    // - system is not NULL and valid (see docstring)
    // - system is valid for the life of this function (documented as not threadsafe)
    // - system will not be modified during the life of this function (documented as not threadsafe)
    unsafe {
        BoxedSystem::with_ref_nonnull(system, |system| {
            // SAFETY:
            // - hittr_status_t is not allocated, so no issues
            unsafe { StatusValue::return_val(system.status) }
        })
    }
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
    let mut sys: *mut System = std::ptr::null_mut();
    assert!(unsafe { hittr_system_new_network(&mut sys as *mut *mut System, 1300) });
    let st = unsafe { hittr_system_status(sys) };
    assert_eq!(st.status, HITTR_STATUS_READY);
    assert_eq!(st.count, 0);
}
