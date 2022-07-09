use std::alloc::{Allocator, GlobalAlloc, Layout, System};
use std::ptr::NonNull;

pub static mut COUNT: usize = 0;

pub struct MeasureMemory;

impl MeasureMemory {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn measure(&self) -> usize {
        unsafe { COUNT }
    }
}

unsafe impl GlobalAlloc for MeasureMemory {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT += layout.pad_to_align().size();
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        COUNT -= layout.pad_to_align().size();
        System.dealloc(ptr, layout);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        COUNT += layout.pad_to_align().size();
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        COUNT -= layout.pad_to_align().size();
        COUNT += new_size;
        System.realloc(ptr, layout, new_size)
    }
}

unsafe impl Allocator for MeasureMemory {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
        unsafe {
            COUNT += layout.pad_to_align().size();
        }
        System.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        COUNT -= layout.pad_to_align().size();
        System.deallocate(ptr, layout)
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
        unsafe {
            COUNT += layout.pad_to_align().size();
        }
        System.allocate_zeroed(layout)
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
        COUNT += new_layout.pad_to_align().size() - old_layout.pad_to_align().size();
        System.grow(ptr, old_layout, new_layout)
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
        COUNT += new_layout.pad_to_align().size() - old_layout.pad_to_align().size();
        System.grow_zeroed(ptr, old_layout, new_layout)
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, std::alloc::AllocError> {
        COUNT -= old_layout.pad_to_align().size() - new_layout.pad_to_align().size();
        System.shrink(ptr, old_layout, new_layout)
    }
}
