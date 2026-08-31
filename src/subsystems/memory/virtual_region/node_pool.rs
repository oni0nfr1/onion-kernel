use core::{mem::MaybeUninit, ptr::NonNull};

use super::tree::VirtualRegionDescriptor;

#[repr(C)]
pub struct RegionNodeSlab {
    pub next: Option<NonNull<RegionNodeSlab>>,
}

#[repr(C)]
pub struct FreeRegionNodeSlot {
    pub next: Option<NonNull<FreeRegionNodeSlot>>,
}

pub struct RegionNodePool {
    pub slabs: Option<NonNull<RegionNodeSlab>>,
    pub free_head: Option<NonNull<FreeRegionNodeSlot>>,
    pub free_count: usize,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl RegionNodePool {
    pub fn new() -> Self {
        todo!()
    }

    pub fn available(&self) -> usize {
        todo!()
    }

    pub fn take(&mut self) -> Option<NonNull<MaybeUninit<VirtualRegionDescriptor>>> {
        todo!()
    }

    /// # Safety
    ///
    /// `node` must belong to this pool, must not be linked into either region
    /// tree, and must not have already been returned.
    pub unsafe fn put(&mut self, node: NonNull<VirtualRegionDescriptor>) {
        todo!()
    }

    /// Adds a page-sized, writable HHDM mapping as a new node slab.
    ///
    /// # Safety
    ///
    /// `page` must point to an exclusively owned, page-aligned physical page
    /// mapped writable for at least one complete page. The physical page becomes owned
    /// by the pool and must remain mapped for the pool's lifetime.
    pub unsafe fn add_slab(&mut self, page: NonNull<u8>) {
        todo!()
    }
}
