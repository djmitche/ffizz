use std::mem;

/// Verify that CType and RType have the same alignment requirements, and that RType is not larger
/// than CType.
///
/// These checks will compile to nothing if the requirements are met, and will compile to
/// `debug_assert!(false)` if they are not met, causing all trait methods to panic.  That should be
/// enough to get someone's attention!
pub(crate) fn check_size_and_alignment<CType: Sized, RType: Sized>() {
    debug_assert!(mem::size_of::<RType>() <= mem::size_of::<CType>());
    debug_assert!(mem::align_of::<RType>() == mem::align_of::<CType>());
}
