//! A global allocator that counts the bytes one thread asks for, on demand.
//!
//! The scaling tests in `test_relationship_scaling` use it as a work counter
//! that wall-clock noise cannot reach: an allocation is deterministic, and the
//! 2.0.0 exclusivity check allocated a set holding every target of the whole
//! junction on every write. Counting is per thread and off by default, so the
//! other tests of this binary, which run in parallel on their own threads, are
//! neither counted nor slowed beyond one thread-local read per allocation.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAllocator;

thread_local! {
    // `const` initialisers and no destructor: reading these never allocates,
    // which is what makes them usable from inside the allocator itself.
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

fn record(size: usize) {
    let _ = COUNTING.try_with(|counting| {
        if counting.get() {
            let _ = BYTES.try_with(|bytes| bytes.set(bytes.get() + size));
        }
    });
}

// SAFETY: every call is forwarded unchanged to `System`; the wrapper only
// adds a thread-local counter that never allocates.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(new_size);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Run `f` and return its result with the number of bytes this thread
/// allocated (or reallocated to) while it ran.
pub fn bytes_allocated_by<R>(f: impl FnOnce() -> R) -> (R, usize) {
    BYTES.with(|bytes| bytes.set(0));
    COUNTING.with(|counting| counting.set(true));
    let result = f();
    COUNTING.with(|counting| counting.set(false));
    (result, BYTES.with(Cell::get))
}
