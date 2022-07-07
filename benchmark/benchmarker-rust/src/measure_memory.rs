use std::alloc::{GlobalAlloc, System};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct MeasureMemory {
    count: AtomicUsize,
}

impl MeasureMemory {
    pub const fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    pub fn measure(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

unsafe impl GlobalAlloc for MeasureMemory {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        self.count
            .fetch_add(layout.pad_to_align().size(), Ordering::SeqCst);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        System.dealloc(ptr, layout);
        self.count
            .fetch_sub(layout.pad_to_align().size(), Ordering::SeqCst);
    }
}
