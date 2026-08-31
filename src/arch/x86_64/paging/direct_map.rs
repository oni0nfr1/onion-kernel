use crate::arch::common::paging::{Address, DirectMap as CommonDirectMap};

use super::address::{PAGE_SIZE, PhysicalAddress, VirtualAddress};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectMapError {
    InvalidPhysicalAddressWidth,
    InvalidPhysicalAddressLimit,
    UnalignedBase,
    UnalignedPhysicalAddressLimit,
    VirtualAddressOverflow,
    CrossesCanonicalAddressBoundary,
}

/// Describes offset-based address conversion within a bounded direct-map span.
///
/// This value describes address arithmetic. Constructing it does not prove
/// that every page in the span is present in the active page tables. Callers
/// must only dereference converted addresses whose physical pages are known to
/// be mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelDirectMap {
    virtual_base: VirtualAddress,
    physical_address_limit: u64,
    max_physical_address_bits: u8,
}

impl KernelDirectMap {
    pub fn new(
        virtual_base: VirtualAddress,
        physical_address_limit: u64,
        max_physical_address_bits: u8,
    ) -> Result<Self, DirectMapError> {
        if PhysicalAddress::new(0, max_physical_address_bits).is_none() {
            return Err(DirectMapError::InvalidPhysicalAddressWidth);
        }
        if virtual_base.value() % PAGE_SIZE != 0 {
            return Err(DirectMapError::UnalignedBase);
        }
        if physical_address_limit % PAGE_SIZE != 0 {
            return Err(DirectMapError::UnalignedPhysicalAddressLimit);
        }

        if physical_address_limit > 0 {
            let last_physical = physical_address_limit - 1;
            PhysicalAddress::new(last_physical, max_physical_address_bits)
                .ok_or(DirectMapError::InvalidPhysicalAddressLimit)?;

            let last_virtual = virtual_base
                .value()
                .checked_add(last_physical)
                .ok_or(DirectMapError::VirtualAddressOverflow)?;
            let last_virtual = VirtualAddress::new(last_virtual)
                .ok_or(DirectMapError::CrossesCanonicalAddressBoundary)?;

            let base_is_upper = virtual_base.value() & (1 << 47) != 0;
            let end_is_upper = last_virtual.value() & (1 << 47) != 0;
            if base_is_upper != end_is_upper {
                return Err(DirectMapError::CrossesCanonicalAddressBoundary);
            }
        }

        Ok(Self {
            virtual_base,
            physical_address_limit,
            max_physical_address_bits,
        })
    }

    pub const fn virtual_base(self) -> VirtualAddress {
        self.virtual_base
    }

    pub const fn physical_address_limit(self) -> u64 {
        self.physical_address_limit
    }

    pub const fn max_physical_address_bits(self) -> u8 {
        self.max_physical_address_bits
    }
}

impl CommonDirectMap for KernelDirectMap {
    type PhysicalAddress = PhysicalAddress;
    type VirtualAddress = VirtualAddress;

    fn physical_to_virtual(&self, address: PhysicalAddress) -> Option<VirtualAddress> {
        if address.value() >= self.physical_address_limit {
            return None;
        }

        let value = self.virtual_base.value().checked_add(address.value())?;
        VirtualAddress::new(value)
    }

    fn virtual_to_physical(&self, address: VirtualAddress) -> Option<PhysicalAddress> {
        let value = address.value().checked_sub(self.virtual_base.value())?;
        if value >= self.physical_address_limit {
            return None;
        }

        PhysicalAddress::new(value, self.max_physical_address_bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::arch::common::paging::DirectMap;

    const MAX_PHYSICAL_ADDRESS_BITS: u8 = 52;
    const DIRECT_MAP_BASE: u64 = 0xffff_8000_0000_0000;
    const DIRECT_MAP_SIZE: u64 = 1 << 46;

    fn direct_map() -> KernelDirectMap {
        KernelDirectMap::new(
            VirtualAddress::new(DIRECT_MAP_BASE).unwrap(),
            DIRECT_MAP_SIZE,
            MAX_PHYSICAL_ADDRESS_BITS,
        )
        .unwrap()
    }

    #[test]
    fn converts_addresses_in_both_directions() {
        let direct_map = direct_map();
        let physical = PhysicalAddress::new(0x1234_5000, MAX_PHYSICAL_ADDRESS_BITS).unwrap();
        let virtual_address = direct_map.physical_to_virtual(physical).unwrap();

        assert_eq!(virtual_address.value(), DIRECT_MAP_BASE + 0x1234_5000);
        assert_eq!(
            direct_map.virtual_to_physical(virtual_address),
            Some(physical)
        );
    }

    #[test]
    fn rejects_addresses_outside_its_span() {
        let direct_map = direct_map();
        let outside = PhysicalAddress::new(DIRECT_MAP_SIZE, MAX_PHYSICAL_ADDRESS_BITS).unwrap();

        assert_eq!(direct_map.physical_to_virtual(outside), None);
        assert_eq!(
            direct_map.virtual_to_physical(
                VirtualAddress::new(DIRECT_MAP_BASE + DIRECT_MAP_SIZE).unwrap()
            ),
            None
        );
    }

    #[test]
    fn rejects_a_span_crossing_the_canonical_hole() {
        let base = VirtualAddress::new(0x0000_7fff_ffff_f000).unwrap();

        assert_eq!(
            KernelDirectMap::new(base, PAGE_SIZE * 2, MAX_PHYSICAL_ADDRESS_BITS),
            Err(DirectMapError::CrossesCanonicalAddressBoundary)
        );
    }
}
