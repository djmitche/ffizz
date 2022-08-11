This trait supports values passed to Rust by pointer.
These values are represented as in C, and always handled as pointers.

C and Rust use different allocators, so it must be unambiguous what memory was allocated by which allocator.
Depending on the use-case, all allocations may occur in one allocator or the other, in which case this distinction is simple.
In more complex use-cases, it may be necessary to include a flag to indicate which allocator "owns" the memory.

# Example

```rust
use ffizz_passby::PassByPointer;

// TODO...

```
