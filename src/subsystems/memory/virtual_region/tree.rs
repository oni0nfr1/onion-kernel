use core::ptr::NonNull;

use crate::arch::common::paging::VirtualPage;

use super::RegionState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RbColor {
    Red,
    Black,
}

#[derive(Debug)]
pub struct RbTreeLinks<P>
where
    P: VirtualPage,
{
    pub parent: Option<NonNull<VirtualRegionDescriptor<P>>>,
    pub left: Option<NonNull<VirtualRegionDescriptor<P>>>,
    pub right: Option<NonNull<VirtualRegionDescriptor<P>>>,
    pub color: RbColor,
}

#[derive(Debug)]
pub struct VirtualRegionDescriptor<P>
where
    P: VirtualPage,
{
    pub start: P,
    pub page_count: usize,
    pub state: RegionState,
    pub address_links: RbTreeLinks<P>,
    pub size_links: RbTreeLinks<P>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionTreeIndex {
    Address,
    FreeSize,
}

pub struct RegionTree<P>
where
    P: VirtualPage,
{
    pub root: Option<NonNull<VirtualRegionDescriptor<P>>>,
    pub len: usize,
    pub index: RegionTreeIndex,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<P> RegionTree<P>
where
    P: VirtualPage,
{
    pub fn new(index: RegionTreeIndex) -> Self {
        todo!()
    }

    /// # Safety
    ///
    /// `node` must point to a live descriptor owned by this allocator and must
    /// not already be linked through this tree's selected link set.
    pub unsafe fn insert(&mut self, node: NonNull<VirtualRegionDescriptor<P>>) {
        todo!()
    }

    /// # Safety
    ///
    /// `node` must be a live descriptor currently linked into this tree.
    pub unsafe fn remove(&mut self, node: NonNull<VirtualRegionDescriptor<P>>) {
        todo!()
    }

    /// # Safety
    ///
    /// `node` must be a live descriptor currently linked into this tree.
    pub unsafe fn predecessor(
        &self,
        node: NonNull<VirtualRegionDescriptor<P>>,
    ) -> Option<NonNull<VirtualRegionDescriptor<P>>> {
        todo!()
    }

    /// # Safety
    ///
    /// `node` must be a live descriptor currently linked into this tree.
    pub unsafe fn successor(
        &self,
        node: NonNull<VirtualRegionDescriptor<P>>,
    ) -> Option<NonNull<VirtualRegionDescriptor<P>>> {
        todo!()
    }

    pub fn find_suitable(
        &self,
        page_count: usize,
        alignment_pages: usize,
    ) -> Option<NonNull<VirtualRegionDescriptor<P>>> {
        todo!()
    }
}
