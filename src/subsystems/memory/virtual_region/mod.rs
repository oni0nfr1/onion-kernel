pub mod node_pool;
pub mod tree;

use crate::arch::x86_64::paging::address::VirtualPage;

use self::{node_pool::RegionNodePool, tree::RegionTree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualRegion {
    start: VirtualPage,
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
impl VirtualRegion {
    pub fn new(start: VirtualPage, page_count: usize) -> Result<Self, VirtualRegionError> {
        todo!()
    }

    pub fn start(self) -> VirtualPage {
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
pub struct VirtualRegionAllocator {
    arena: VirtualRegion,
    address_tree: RegionTree,
    free_size_tree: RegionTree,
    node_pool: RegionNodePool,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl VirtualRegionAllocator {
    pub fn new(
        arena: VirtualRegion,
        node_pool: RegionNodePool,
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
    ) -> Result<VirtualRegion, VirtualRegionError> {
        todo!()
    }

    pub fn release(&mut self, region: VirtualRegion) -> Result<(), VirtualRegionError> {
        todo!()
    }

    pub fn node_pool(&self) -> &RegionNodePool {
        todo!()
    }

    pub fn node_pool_mut(&mut self) -> &mut RegionNodePool {
        todo!()
    }
}
