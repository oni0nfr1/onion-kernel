#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapMetadataLayout {
    pub region_descriptor_count: usize,
    pub bitmap_bytes: usize,
    pub total_bytes: usize,
    pub alignment: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapError {
    NoUsableMemory,
    MetadataSpaceUnavailable,
    AddressOverflow,
    SizeOverflow,
    InvalidMemoryMap,
}

#[expect(dead_code, reason = "implementation scaffold")]
pub struct LimineMemoryBootstrap {
    memory_map: &'static MemmapResponse,
    hhdm_offset: u64,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl LimineMemoryBootstrap {
    pub fn new(memory_map: &'static MemmapResponse, hhdm_offset: u64) -> Self {
        todo!()
    }

    pub fn metadata_layout(&self) -> Result<BootstrapMetadataLayout, BootstrapError> {
        todo!()
    }

    /// # Safety
    ///
    /// The memory-map response and HHDM must remain valid for the kernel's
    /// lifetime. The caller must ensure that memory selected for allocator
    /// metadata does not alias any live object.
    pub unsafe fn initialize(self) -> Result<MemoryManager<'static>, BootstrapError> {
        todo!()
    }
}
use limine::request::MemmapResponse;

use super::manager::MemoryManager;
