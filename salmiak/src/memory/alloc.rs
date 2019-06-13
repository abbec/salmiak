use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
    sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(target_arch = "aarch64")]
pub fn create_child_allocator<T: Allocator>(parent: Option<&dyn Allocator>, size: usize) -> T {
    let mem = match parent {
        Some(a) => unsafe { a.alloc(Layout::from_size_align_unchecked(size, 16)) as usize },
        None => unsafe {
            crate::ALLOCATOR.alloc(Layout::from_size_align_unchecked(size, 16)) as usize
        },
    } as usize;

    T::new(mem, mem + size)
}

pub(crate) struct OriginAllocator {
    inner: BumpAllocator,
}

unsafe impl GlobalAlloc for OriginAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.dealloc(ptr, layout)
    }
}

impl OriginAllocator {
    pub const fn new() -> Self {
        OriginAllocator {
            inner: BumpAllocator::empty(),
        }
    }

    pub fn initialize(&mut self, start: usize, size: usize) {
        self.inner = BumpAllocator::new(start, size);
    }
}

pub trait Allocator {
    fn new(start: usize, size: usize) -> Self
    where
        Self: Sized;
    fn alloc(&self, layout: Layout) -> *mut u8;
    fn dealloc(&self, _ptr: *mut u8, _layout: Layout);
}

/// Align downwards. Returns the greatest x with alignment `align`
/// so that x <= addr. The alignment must be a power of 2.
fn align_down(addr: usize, align: usize) -> usize {
    if align.is_power_of_two() {
        addr & !(align - 1)
    } else if align == 0 {
        addr
    } else {
        panic!("`align` must be a power of 2");
    }
}

/// Align upwards. Returns the smallest x with alignment `align`
/// so that x >= addr. The alignment must be a power of 2.
pub fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr + align - 1, align)
}

/// A simple allocator that allocates memory linearly and ignores freed memory.
#[derive(Debug)]
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: AtomicUsize,
}

impl BumpAllocator {
    const fn empty() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: AtomicUsize::new(0),
        }
    }
}

impl Allocator for BumpAllocator {
    fn new(heap_start: usize, size: usize) -> Self {
        Self {
            heap_start,
            heap_end: heap_start + size,
            next: AtomicUsize::new(heap_start),
        }
    }

    fn alloc(&self, layout: Layout) -> *mut u8 {
        loop {
            // load current state of the `next` field
            let current_next = self.next.load(Ordering::Relaxed);
            let alloc_start = align_up(current_next, layout.align());
            let alloc_end = alloc_start.saturating_add(layout.size());

            if alloc_end <= self.heap_end {
                // update the `next` pointer if it still has the value `current_next`
                let next_now =
                    self.next
                        .compare_and_swap(current_next, alloc_end, Ordering::Relaxed);
                if next_now == current_next {
                    // next address was successfully updated, allocation succeeded
                    return alloc_start as *mut u8;
                }
            } else {
                return null_mut();
            }
        }
    }

    fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // do nothing, leak memory
    }
}
