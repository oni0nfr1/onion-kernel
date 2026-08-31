use core::{iter::FusedIterator, slice};

use limine::memmap::{self, Entry};

use crate::boot::{
    AddressDecoder,
    limine::requests::MEMMAP_REQUEST,
    protocol::memory::{MemoryMap, MemoryMapEntryError, MemoryRegionKind, PhysicalMemoryRegion},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimineMemoryMapError {
    MissingResponse,
}

/// A reusable, allocation-free view of Limine's memory-map response.
///
/// The view borrows the address decoder and retains only Limine-owned entry
/// references. Each iteration constructs bootloader-independent region values
/// on demand.
pub struct LimineMemoryMap<'a, D>
where
    D: AddressDecoder + ?Sized,
{
    entries: &'static [&'static Entry],
    decoder: &'a D,
}

impl<'a, D> Clone for LimineMemoryMap<'a, D>
where
    D: AddressDecoder + ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<D> Copy for LimineMemoryMap<'_, D> where D: AddressDecoder + ?Sized {}

impl<D> MemoryMap for LimineMemoryMap<'_, D>
where
    D: AddressDecoder + ?Sized,
{
    type Address = D::Address;
    type RegionIter<'a>
        = MemoryRegionIter<'a, D>
    where
        Self: 'a;

    fn iter(&self) -> Self::RegionIter<'_> {
        MemoryRegionIter {
            entries: self.entries.iter(),
            decoder: self.decoder,
        }
    }
}

impl<'a, D> IntoIterator for LimineMemoryMap<'a, D>
where
    D: AddressDecoder + ?Sized,
{
    type Item = Result<PhysicalMemoryRegion<D::Address>, MemoryMapEntryError>;
    type IntoIter = MemoryRegionIter<'a, D>;

    fn into_iter(self) -> Self::IntoIter {
        MemoryRegionIter {
            entries: self.entries.iter(),
            decoder: self.decoder,
        }
    }
}

impl<'map, 'decoder, D> IntoIterator for &'map LimineMemoryMap<'decoder, D>
where
    D: AddressDecoder + ?Sized,
    'decoder: 'map,
{
    type Item = Result<PhysicalMemoryRegion<D::Address>, MemoryMapEntryError>;
    type IntoIter = MemoryRegionIter<'map, D>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct MemoryRegionIter<'a, D>
where
    D: AddressDecoder + ?Sized,
{
    entries: slice::Iter<'static, &'static Entry>,
    decoder: &'a D,
}

impl<'a, D> Clone for MemoryRegionIter<'a, D>
where
    D: AddressDecoder + ?Sized,
{
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            decoder: self.decoder,
        }
    }
}

impl<D> Iterator for MemoryRegionIter<'_, D>
where
    D: AddressDecoder + ?Sized,
{
    type Item = Result<PhysicalMemoryRegion<D::Address>, MemoryMapEntryError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries
            .next()
            .map(|entry| convert_entry(entry, self.decoder))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.entries.size_hint()
    }
}

impl<D> ExactSizeIterator for MemoryRegionIter<'_, D> where D: AddressDecoder + ?Sized {}
impl<D> FusedIterator for MemoryRegionIter<'_, D> where D: AddressDecoder + ?Sized {}

pub fn memory_map<D>(decoder: &D) -> Result<LimineMemoryMap<'_, D>, LimineMemoryMapError>
where
    D: AddressDecoder + ?Sized,
{
    let response = MEMMAP_REQUEST
        .response()
        .ok_or(LimineMemoryMapError::MissingResponse)?;

    Ok(LimineMemoryMap {
        entries: response.entries(),
        decoder,
    })
}

fn convert_entry<D>(
    entry: &Entry,
    decoder: &D,
) -> Result<PhysicalMemoryRegion<D::Address>, MemoryMapEntryError>
where
    D: AddressDecoder + ?Sized,
{
    let start = decoder
        .decode(entry.base)
        .ok_or(MemoryMapEntryError::InvalidAddress)?;
    PhysicalMemoryRegion::new(start, entry.length, region_kind(entry.type_))
        .map_err(MemoryMapEntryError::InvalidRegion)
}

const fn region_kind(kind: u64) -> MemoryRegionKind {
    match kind {
        memmap::MEMMAP_USABLE => MemoryRegionKind::Usable,
        memmap::MEMMAP_RESERVED => MemoryRegionKind::Reserved,
        memmap::MEMMAP_ACPI_RECLAIMABLE => MemoryRegionKind::AcpiReclaimable,
        memmap::MEMMAP_ACPI_NVS => MemoryRegionKind::AcpiNvs,
        memmap::MEMMAP_BAD_MEMORY => MemoryRegionKind::BadMemory,
        memmap::MEMMAP_BOOTLOADER_RECLAIMABLE => MemoryRegionKind::BootloaderReclaimable,
        memmap::MEMMAP_EXECUTABLE_AND_MODULES => MemoryRegionKind::KernelAndModules,
        memmap::MEMMAP_FRAMEBUFFER => MemoryRegionKind::Framebuffer,
        memmap::MEMMAP_MAPPED_RESERVED => MemoryRegionKind::MappedReserved,
        unknown => MemoryRegionKind::Unknown(unknown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::arch::common::paging::Address;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct TestAddress(u64);

    impl Address for TestAddress {
        fn value(self) -> u64 {
            self.0
        }
    }

    struct TestAddressDecoder;

    impl AddressDecoder for TestAddressDecoder {
        type Address = TestAddress;

        fn decode(&self, value: u64) -> Option<Self::Address> {
            (value < 0x1_0000).then_some(TestAddress(value))
        }
    }

    #[test]
    fn converts_a_limine_entry_without_retaining_it() {
        let entry = Entry {
            base: 0x2000,
            length: 0x3000,
            type_: memmap::MEMMAP_USABLE,
        };

        let region = convert_entry(&entry, &TestAddressDecoder).unwrap();

        assert_eq!(region.start(), TestAddress(0x2000));
        assert_eq!(region.byte_len(), 0x3000);
        assert_eq!(region.kind(), MemoryRegionKind::Usable);
    }

    #[test]
    fn preserves_unknown_region_kinds() {
        let entry = Entry {
            base: 0x2000,
            length: 0x1000,
            type_: 42,
        };

        let region = convert_entry(&entry, &TestAddressDecoder).unwrap();

        assert_eq!(region.kind(), MemoryRegionKind::Unknown(42));
    }

    #[test]
    fn rejects_an_address_outside_the_decoder_domain() {
        let entry = Entry {
            base: 0x1_0000,
            length: 0x1000,
            type_: memmap::MEMMAP_RESERVED,
        };

        assert_eq!(
            convert_entry(&entry, &TestAddressDecoder),
            Err(MemoryMapEntryError::InvalidAddress),
        );
    }
}
