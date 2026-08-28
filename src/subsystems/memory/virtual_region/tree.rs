use core::ptr::NonNull;

use crate::arch::x86_64::paging::address::VirtualPage;

use super::RegionState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RbColor {
    Red,
    Black,
}

#[derive(Debug)]
pub struct RbTreeLinks {
    pub parent: Option<NonNull<VirtualRegionDescriptor>>,
    pub left: Option<NonNull<VirtualRegionDescriptor>>,
    pub right: Option<NonNull<VirtualRegionDescriptor>>,
    pub color: RbColor,
}

#[derive(Debug)]
pub struct VirtualRegionDescriptor {
    pub start: VirtualPage,
    pub page_count: usize,
    pub state: RegionState,
    pub address_links: RbTreeLinks,
    pub size_links: RbTreeLinks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionTreeIndex {
    Address,
    FreeSize,
}

pub struct RegionTree {
    pub root: Option<NonNull<VirtualRegionDescriptor>>,
    pub len: usize,
    pub index: RegionTreeIndex,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl RegionTree {
    pub fn new(index: RegionTreeIndex) -> Self {
        todo!()
    }

    /// # Safety
    ///
    /// `node` must point to a live descriptor owned by this allocator and must
    /// not already be linked through this tree's selected link set.
    pub unsafe fn insert(&mut self, node: NonNull<VirtualRegionDescriptor>) {
        todo!()
    }

    /// # Safety
    ///
    /// `node` must be a live descriptor currently linked into this tree.
    pub unsafe fn remove(&mut self, node: NonNull<VirtualRegionDescriptor>) {
        todo!()
    }

    /// # Safety
    ///
    /// `node` must be a live descriptor currently linked into this tree.
    pub unsafe fn predecessor(
        &self,
        node: NonNull<VirtualRegionDescriptor>,
    ) -> Option<NonNull<VirtualRegionDescriptor>> {
        todo!()
    }

    /// # Safety
    ///
    /// `node` must be a live descriptor currently linked into this tree.
    pub unsafe fn successor(
        &self,
        node: NonNull<VirtualRegionDescriptor>,
    ) -> Option<NonNull<VirtualRegionDescriptor>> {
        todo!()
    }

    pub fn find_suitable(
        &self,
        page_count: usize,
        alignment_pages: usize,
    ) -> Option<NonNull<VirtualRegionDescriptor>> {
        todo!()
    }
}
