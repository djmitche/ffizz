/// An implementation of Vec::into_raw_parts, which is still unstable.  Returns ptr, len, cap.
pub(crate) fn vec_into_raw_parts<T>(vec: Vec<T>) -> (*mut T, usize, usize) {
    // emulate Vec::into_raw_parts():
    // - disable dropping the Vec with ManuallyDrop
    // - extract ptr, len, and capacity using those methods
    let mut vec = std::mem::ManuallyDrop::new(vec);
    (vec.as_mut_ptr(), vec.len(), vec.capacity())
}
