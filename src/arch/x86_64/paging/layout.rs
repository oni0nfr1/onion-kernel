use crate::arch::common::paging::{KernelVirtualMemoryLayout, VirtualPageRange};

use super::address::{VirtualAddress, VirtualPage};

const PAGES_PER_PML4_ENTRY: usize = 1 << 27;

const DIRECT_MAP_START: u64 = 0xffff_8000_0000_0000;
const KERNEL_HEAP_START: u64 = 0xffff_c000_0000_0000;
const KERNEL_MAPPING_START: u64 = 0xffff_d000_0000_0000;
const MMIO_START: u64 = 0xffff_e000_0000_0000;
const KERNEL_STACK_START: u64 = 0xffff_e800_0000_0000;
const PER_CPU_START: u64 = 0xffff_f000_0000_0000;
const KERNEL_IMAGE_START: u64 = 0xffff_ff80_0000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FourLevelKernelLayout {
    direct_map: VirtualPageRange<VirtualPage>,
    kernel_heap: VirtualPageRange<VirtualPage>,
    kernel_mapping: VirtualPageRange<VirtualPage>,
    mmio: VirtualPageRange<VirtualPage>,
    kernel_stack: VirtualPageRange<VirtualPage>,
    per_cpu: VirtualPageRange<VirtualPage>,
    kernel_image: VirtualPageRange<VirtualPage>,
}

impl FourLevelKernelLayout {
    pub fn new() -> Option<Self> {
        Some(Self {
            direct_map: region(DIRECT_MAP_START, 128)?,
            kernel_heap: region(KERNEL_HEAP_START, 32)?,
            kernel_mapping: region(KERNEL_MAPPING_START, 32)?,
            mmio: region(MMIO_START, 16)?,
            kernel_stack: region(KERNEL_STACK_START, 16)?,
            per_cpu: region(PER_CPU_START, 16)?,
            kernel_image: region(KERNEL_IMAGE_START, 1)?,
        })
    }
}

impl KernelVirtualMemoryLayout for FourLevelKernelLayout {
    type VirtualPage = VirtualPage;

    fn direct_map_region(&self) -> VirtualPageRange<Self::VirtualPage> {
        self.direct_map
    }

    fn kernel_heap_region(&self) -> VirtualPageRange<Self::VirtualPage> {
        self.kernel_heap
    }

    fn kernel_mapping_region(&self) -> VirtualPageRange<Self::VirtualPage> {
        self.kernel_mapping
    }

    fn mmio_region(&self) -> VirtualPageRange<Self::VirtualPage> {
        self.mmio
    }

    fn kernel_stack_region(&self) -> VirtualPageRange<Self::VirtualPage> {
        self.kernel_stack
    }

    fn per_cpu_region(&self) -> VirtualPageRange<Self::VirtualPage> {
        self.per_cpu
    }

    fn kernel_image_region(&self) -> VirtualPageRange<Self::VirtualPage> {
        self.kernel_image
    }
}

fn region(start: u64, pml4_entry_count: usize) -> Option<VirtualPageRange<VirtualPage>> {
    let start = VirtualPage::from_start_address(VirtualAddress::new(start)?)?;
    let page_count = PAGES_PER_PML4_ENTRY.checked_mul(pml4_entry_count)?;
    VirtualPageRange::new(start, page_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::arch::common::paging::{Address, Page, VirtualPageRange};
    use crate::arch::x86_64::paging::address::PhysicalPage;

    fn start_address(range: VirtualPageRange<VirtualPage>) -> u64 {
        range.start().start_address().value()
    }

    #[test]
    fn constructs_the_documented_regions() {
        let layout = FourLevelKernelLayout::new().unwrap();

        assert_eq!(start_address(layout.direct_map_region()), DIRECT_MAP_START);
        assert_eq!(layout.direct_map_region().page_count(), 128 << 27);
        assert_eq!(
            start_address(layout.kernel_heap_region()),
            KERNEL_HEAP_START
        );
        assert_eq!(
            start_address(layout.kernel_mapping_region()),
            KERNEL_MAPPING_START
        );
        assert_eq!(start_address(layout.mmio_region()), MMIO_START);
        assert_eq!(
            start_address(layout.kernel_stack_region()),
            KERNEL_STACK_START
        );
        assert_eq!(start_address(layout.per_cpu_region()), PER_CPU_START);
        assert_eq!(
            start_address(layout.kernel_image_region()),
            KERNEL_IMAGE_START
        );
    }

    #[test]
    fn kernel_image_reaches_the_end_of_the_address_space() {
        let layout = FourLevelKernelLayout::new().unwrap();
        let kernel_image = layout.kernel_image_region();

        assert_eq!(kernel_image.page_count(), PAGES_PER_PML4_ENTRY);
        assert_eq!(
            kernel_image.last().unwrap().start_address().value(),
            u64::MAX - (PhysicalPage::SIZE - 1)
        );
        assert_eq!(kernel_image.end(), None);
    }
}
