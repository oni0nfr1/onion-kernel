pub mod free_block;
pub mod global;

use core::{alloc::Layout, ptr::NonNull};

use super::manager::MemoryManager;
use free_block::FreeBlockTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapAllocationError {
    InvalidLayout,
    AddressOverflow,
    OutOfMemory,
    PageAllocationFailed,
}

#[expect(dead_code, reason = "implementation scaffold")]
pub struct KernelHeapInner {
    address_tree: FreeBlockTree,
    free_size_tree: FreeBlockTree,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl KernelHeapInner {
    pub fn new() -> Self {
        todo!()
    }

    pub fn allocate(
        &mut self,
        layout: Layout,
        memory: &mut MemoryManager<'_>,
    ) -> Result<NonNull<u8>, HeapAllocationError> {
        todo!()
    }

    /// # Safety
    ///
    /// `pointer` and `layout` must describe a currently live allocation
    /// returned by this heap, and the allocation must no longer be used.
    pub unsafe fn deallocate(
        &mut self,
        pointer: NonNull<u8>,
        layout: Layout,
        memory: &mut MemoryManager<'_>,
    ) {
        todo!()
    }
}

/// The lock type is left generic until the kernel synchronization primitive is
/// selected. It is expected to protect a `KernelHeapInner`.
#[expect(dead_code, reason = "implementation scaffold")]
pub struct KernelHeap<L> {
    inner: L,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<L> KernelHeap<L> {
    pub fn new(inner: L) -> Self {
        todo!()
    }

    pub fn into_inner(self) -> L {
        todo!()
    }
}
