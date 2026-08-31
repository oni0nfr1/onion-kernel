use crate::arch::common::paging::{
    Address as CommonAddress, Page as CommonPage, PhysicalPage as CommonPhysicalPage,
    VirtualPage as CommonVirtualPage,
};

use core::arch::x86_64::__cpuid;

pub const PAGE_SIZE: u64 = 4096;

/// Reads the processor's architectural physical-address width.
pub fn max_physical_address_bits() -> Option<u8> {
    let maximum_extended_leaf = __cpuid(0x8000_0000).eax;
    if maximum_extended_leaf < 0x8000_0008 {
        return None;
    }

    let bits = __cpuid(0x8000_0008).eax as u8;
    (12..=52).contains(&bits).then_some(bits)
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    pub const fn new(value: u64, max_physical_address_bits: u8) -> Option<Self> {
        match max_physical_address_bits {
            1..=63 if value < (1u64 << max_physical_address_bits) => Some(Self(value)),
            64 => Some(Self(value)),
            _ => None,
        }
    }
}

impl CommonAddress for PhysicalAddress {
    fn value(self) -> u64 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(u64);

impl VirtualAddress {
    pub const fn new(value: u64) -> Option<Self> {
        let upper = value >> 48;
        let expected_upper = if value & (1 << 47) == 0 {
            0x0000
        } else {
            0xffff
        };

        if upper == expected_upper {
            Some(Self(value))
        } else {
            None
        }
    }
}

impl CommonAddress for VirtualAddress {
    fn value(self) -> u64 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalPage(u64);

impl PhysicalPage {
    pub fn from_start_address(address: PhysicalAddress) -> Option<Self> {
        if address.value() % PAGE_SIZE == 0 {
            Some(Self(address.value() / PAGE_SIZE))
        } else {
            None
        }
    }
}

impl CommonPage for PhysicalPage {
    type Address = PhysicalAddress;

    const SIZE: u64 = PAGE_SIZE;

    fn start_address(self) -> Self::Address {
        // A PhysicalPage can only be constructed from a validated, page-aligned
        // PhysicalAddress or by checked_add, so this multiplication cannot exceed
        // the MAXPHYADDR limit used to construct it.
        PhysicalAddress(self.0 * PAGE_SIZE)
    }
}

impl CommonPhysicalPage for PhysicalPage {
    type ArithmeticContext = u8;

    fn containing_address(address: Self::Address) -> Self {
        Self(address.value() / PAGE_SIZE)
    }

    fn checked_add(self, page_count: usize, context: Self::ArithmeticContext) -> Option<Self> {
        let page_count = u64::try_from(page_count).ok()?;
        let page_number = self.0.checked_add(page_count)?;
        let start_address = page_number.checked_mul(PAGE_SIZE)?;

        PhysicalAddress::new(start_address, context)?;
        Some(Self(page_number))
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualPage(u64);

impl VirtualPage {
    const TABLE_INDEX_MASK: u64 = 0b111111111;

    pub fn from_start_address(address: VirtualAddress) -> Option<Self> {
        if address.value() % PAGE_SIZE == 0 {
            Some(Self(address.value() / PAGE_SIZE))
        } else {
            None
        }
    }

    pub const fn level4_index(self) -> usize {
        ((self.0 >> 27) & Self::TABLE_INDEX_MASK) as usize
    }

    pub const fn level3_index(self) -> usize {
        ((self.0 >> 18) & Self::TABLE_INDEX_MASK) as usize
    }

    pub const fn level2_index(self) -> usize {
        ((self.0 >> 9) & Self::TABLE_INDEX_MASK) as usize
    }

    pub const fn level1_index(self) -> usize {
        (self.0 & Self::TABLE_INDEX_MASK) as usize
    }
}

impl CommonPage for VirtualPage {
    type Address = VirtualAddress;

    const SIZE: u64 = PAGE_SIZE;

    fn start_address(self) -> Self::Address {
        VirtualAddress(self.0 * PAGE_SIZE)
    }
}

impl CommonVirtualPage for VirtualPage {
    fn checked_add(self, page_count: usize) -> Option<Self> {
        let page_count = u64::try_from(page_count).ok()?;
        let page_number = self.0.checked_add(page_count)?;
        let start_address = page_number.checked_mul(PAGE_SIZE)?;

        VirtualAddress::new(start_address)?;
        Some(Self(page_number))
    }

    fn is_valid_range(self, page_count: usize) -> bool {
        let Some(last_offset) = page_count.checked_sub(1) else {
            return true;
        };
        let Some(last_page) = self.checked_add(last_offset) else {
            return false;
        };

        let start_is_upper_half = self.start_address().value() & (1 << 47) != 0;
        let end_is_upper_half = last_page.start_address().value() & (1 << 47) != 0;
        start_is_upper_half == end_is_upper_half
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::arch::common::paging::VirtualPageRange;

    fn page(address: u64) -> VirtualPage {
        let address = VirtualAddress::new(address).unwrap();
        VirtualPage::from_start_address(address).unwrap()
    }

    #[test]
    fn accepts_the_complete_lower_canonical_half() {
        let range = VirtualPageRange::new(page(0), 1 << 35).unwrap();

        assert_eq!(range.last(), Some(page(0x0000_7fff_ffff_f000)));
        assert_eq!(range.end(), None);
    }

    #[test]
    fn rejects_a_range_crossing_the_canonical_hole() {
        assert!(VirtualPageRange::new(page(0x0000_7fff_ffff_f000), 2).is_none());
    }
}
