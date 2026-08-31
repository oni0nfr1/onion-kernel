//! Physical memory-map values consumed by the memory subsystem.

use crate::arch::common::paging::Address;

/// Bootloader-independent access to a reusable physical memory-map view.
///
/// Every call to [`MemoryMap::iter`] must yield the same immutable sequence of
/// regions and conversion results while this value exists.
pub trait MemoryMap {
    type Address: Address;
    type RegionIter<'a>: Clone
        + ExactSizeIterator<Item = Result<PhysicalMemoryRegion<Self::Address>, MemoryMapEntryError>>
    where
        Self: 'a;

    fn iter(&self) -> Self::RegionIter<'_>;

    fn len(&self) -> usize {
        self.iter().len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMapEntryError {
    InvalidAddress,
    InvalidRegion(PhysicalMemoryRegionError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    BadMemory,
    BootloaderReclaimable,
    KernelAndModules,
    Framebuffer,
    MappedReserved,
    Unknown(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalMemoryRegionError {
    Empty,
    AddressOverflow,
}

/// A physical memory-map entry copied out of a boot-protocol response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalMemoryRegion<P>
where
    P: Address,
{
    start: P,
    byte_len: u64,
    kind: MemoryRegionKind,
}

impl<P> PhysicalMemoryRegion<P>
where
    P: Address,
{
    pub fn new(
        start: P,
        byte_len: u64,
        kind: MemoryRegionKind,
    ) -> Result<Self, PhysicalMemoryRegionError> {
        if byte_len == 0 {
            return Err(PhysicalMemoryRegionError::Empty);
        }
        start
            .value()
            .checked_add(byte_len)
            .ok_or(PhysicalMemoryRegionError::AddressOverflow)?;

        Ok(Self {
            start,
            byte_len,
            kind,
        })
    }

    pub fn start(&self) -> P {
        self.start
    }

    pub fn byte_len(&self) -> u64 {
        self.byte_len
    }

    pub fn kind(&self) -> MemoryRegionKind {
        self.kind
    }
}
