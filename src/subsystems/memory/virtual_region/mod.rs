pub mod node_pool;
pub mod tree;

use crate::arch::common::paging::VirtualPage;

use self::{node_pool::RegionNodePool, tree::RegionTree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualRegion<P>
where
    P: VirtualPage,
{
    start: P,
    page_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionState {
    Free,
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualRegionError {
    NoSuitableRegion,
    InvalidAlignment,
    AddressOverflow,
    NodePoolExhausted { additional_nodes: usize },
    InvalidRegion,
    RegionNotReserved,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<P> VirtualRegion<P>
where
    P: VirtualPage,
{
    pub fn new(start: P, page_count: usize) -> Result<Self, VirtualRegionError> {
        todo!()
    }

    pub fn start(self) -> P {
        todo!()
    }

    pub fn page_count(self) -> usize {
        todo!()
    }

    pub fn is_empty(self) -> bool {
        todo!()
    }
}

#[expect(dead_code, reason = "implementation scaffold")]
pub struct VirtualRegionAllocator<P>
where
    P: VirtualPage,
{
    arena: VirtualRegion<P>,
    address_tree: RegionTree<P>,
    free_size_tree: RegionTree<P>,
    node_pool: RegionNodePool<P>,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<P> VirtualRegionAllocator<P>
where
    P: VirtualPage,
{
    pub fn new(
        arena: VirtualRegion<P>,
        node_pool: RegionNodePool<P>,
    ) -> Result<Self, VirtualRegionError> {
        todo!()
    }

    pub fn additional_nodes_required(
        &self,
        page_count: usize,
        alignment_pages: usize,
    ) -> Result<usize, VirtualRegionError> {
        todo!()
    }

    pub fn reserve(
        &mut self,
        page_count: usize,
        alignment_pages: usize,
    ) -> Result<VirtualRegion<P>, VirtualRegionError> {
        todo!()
    }

    pub fn release(&mut self, region: VirtualRegion<P>) -> Result<(), VirtualRegionError> {
        todo!()
    }

    pub fn node_pool(&self) -> &RegionNodePool<P> {
        todo!()
    }

    pub fn node_pool_mut(&mut self) -> &mut RegionNodePool<P> {
        todo!()
    }
}
