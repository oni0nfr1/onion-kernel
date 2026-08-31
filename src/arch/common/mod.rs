//! Architecture-neutral contracts implemented by concrete architecture code.
//!
//! This module must not depend on modules such as `arch::x86_64`.

pub mod paging;

pub use paging::{
    Address, DirectMap, KernelVirtualMemoryLayout, MapError, MappingPermissions, Page, PageMapper,
    PageTablePageProvider, PhysicalPage, UnmapError, VirtualPage, VirtualPageRange,
};
