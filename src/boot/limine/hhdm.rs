use crate::{
    arch::common::paging::{Address, DirectMap},
    boot::{AddressDecoder, limine::requests::HHDM_REQUEST},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimineHhdmError {
    MissingResponse,
    InvalidOffset,
}

/// Address conversion through the HHDM installed by Limine.
///
/// This value describes Limine's offset arithmetic. It does not prove that an
/// arbitrary physical address is backed by memory or included in the HHDM;
/// callers must only convert addresses belonging to resources that Limine
/// documents as accessible through that mapping.
pub struct LimineHhdm<P, V>
where
    P: AddressDecoder,
    V: AddressDecoder,
{
    offset: u64,
    virtual_base: V::Address,
    physical_address_decoder: P,
    virtual_address_decoder: V,
}

impl<P, V> LimineHhdm<P, V>
where
    P: AddressDecoder,
    V: AddressDecoder,
{
    pub fn new(
        physical_address_decoder: P,
        virtual_address_decoder: V,
    ) -> Result<Self, LimineHhdmError> {
        let response = HHDM_REQUEST
            .response()
            .ok_or(LimineHhdmError::MissingResponse)?;

        Self::from_offset(
            response.offset,
            physical_address_decoder,
            virtual_address_decoder,
        )
    }

    pub fn virtual_base(&self) -> V::Address {
        self.virtual_base
    }

    fn from_offset(
        offset: u64,
        physical_address_decoder: P,
        virtual_address_decoder: V,
    ) -> Result<Self, LimineHhdmError> {
        let virtual_base = virtual_address_decoder
            .decode(offset)
            .ok_or(LimineHhdmError::InvalidOffset)?;

        Ok(Self {
            offset,
            virtual_base,
            physical_address_decoder,
            virtual_address_decoder,
        })
    }
}

impl<P, V> DirectMap for LimineHhdm<P, V>
where
    P: AddressDecoder,
    V: AddressDecoder,
{
    type PhysicalAddress = P::Address;
    type VirtualAddress = V::Address;

    fn physical_to_virtual(&self, address: Self::PhysicalAddress) -> Option<Self::VirtualAddress> {
        let value = self.offset.checked_add(address.value())?;
        self.virtual_address_decoder.decode(value)
    }

    fn virtual_to_physical(&self, address: Self::VirtualAddress) -> Option<Self::PhysicalAddress> {
        let value = address.value().checked_sub(self.offset)?;
        self.physical_address_decoder.decode(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::arch::x86_64::paging::address::{PhysicalAddress, VirtualAddress};

    const MAX_PHYSICAL_ADDRESS_BITS: u8 = 52;
    const HHDM_OFFSET: u64 = 0xffff_8000_0000_0000;

    #[derive(Debug, Clone, Copy)]
    struct PhysicalAddressDecoder;

    impl AddressDecoder for PhysicalAddressDecoder {
        type Address = PhysicalAddress;

        fn decode(&self, value: u64) -> Option<Self::Address> {
            PhysicalAddress::new(value, MAX_PHYSICAL_ADDRESS_BITS)
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct VirtualAddressDecoder;

    impl AddressDecoder for VirtualAddressDecoder {
        type Address = VirtualAddress;

        fn decode(&self, value: u64) -> Option<Self::Address> {
            VirtualAddress::new(value)
        }
    }

    fn hhdm() -> LimineHhdm<PhysicalAddressDecoder, VirtualAddressDecoder> {
        LimineHhdm::from_offset(HHDM_OFFSET, PhysicalAddressDecoder, VirtualAddressDecoder).unwrap()
    }

    #[test]
    fn converts_addresses_in_both_directions() {
        let hhdm = hhdm();
        let physical = PhysicalAddress::new(0x1234_5000, MAX_PHYSICAL_ADDRESS_BITS).unwrap();
        let virtual_address = hhdm.physical_to_virtual(physical).unwrap();

        assert_eq!(virtual_address.value(), HHDM_OFFSET + 0x1234_5000);
        assert_eq!(hhdm.virtual_to_physical(virtual_address), Some(physical));
    }

    #[test]
    fn rejects_virtual_addresses_below_the_hhdm() {
        let hhdm = hhdm();
        let address = VirtualAddress::new(0x1000).unwrap();

        assert_eq!(hhdm.virtual_to_physical(address), None);
    }

    #[test]
    fn rejects_an_invalid_offset() {
        assert!(matches!(
            LimineHhdm::from_offset(
                0x0000_8000_0000_0000,
                PhysicalAddressDecoder,
                VirtualAddressDecoder,
            ),
            Err(LimineHhdmError::InvalidOffset)
        ));
    }
}
