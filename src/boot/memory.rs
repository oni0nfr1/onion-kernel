//! Allocation-free preparation of boot memory-map data.

use core::marker::PhantomData;

use crate::{
    arch::common::paging::{Address as _, PhysicalPage, PhysicalPageRange},
    boot::protocol::memory::{
        MemoryMap, MemoryMapEntryError, MemoryRegionKind, PhysicalMemoryRegion,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsablePageRangeError {
    MemoryMap(MemoryMapEntryError),
    AddressOverflow,
    InvalidPageSize,
    PageCountOverflow,
    InvalidPageRange,
}

/// A reusable view of the complete physical pages contained in usable memory.
pub struct UsablePhysicalPageRanges<'a, M, P>
where
    M: MemoryMap<Address = P::Address> + ?Sized,
    P: PhysicalPage,
{
    memory_map: &'a M,
    arithmetic_context: P::ArithmeticContext,
    page: PhantomData<fn() -> P>,
}

impl<'a, M, P> Clone for UsablePhysicalPageRanges<'a, M, P>
where
    M: MemoryMap<Address = P::Address> + ?Sized,
    P: PhysicalPage,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<M, P> Copy for UsablePhysicalPageRanges<'_, M, P>
where
    M: MemoryMap<Address = P::Address> + ?Sized,
    P: PhysicalPage,
{
}

impl<'a, M, P> UsablePhysicalPageRanges<'a, M, P>
where
    M: MemoryMap<Address = P::Address> + ?Sized,
    P: PhysicalPage,
{
    pub fn new(
        memory_map: &'a M,
        arithmetic_context: P::ArithmeticContext,
    ) -> Result<Self, UsablePageRangeError> {
        let ranges = Self {
            memory_map,
            arithmetic_context,
            page: PhantomData,
        };

        for region in memory_map.iter() {
            let region = region.map_err(UsablePageRangeError::MemoryMap)?;
            complete_usable_pages::<P>(region, arithmetic_context)?;
        }

        Ok(ranges)
    }

    pub fn iter(&self) -> UsablePhysicalPageRangeIter<'_, M, P> {
        UsablePhysicalPageRangeIter {
            regions: self.memory_map.iter(),
            arithmetic_context: self.arithmetic_context,
            page: PhantomData,
        }
    }
}

impl<'a, M, P> IntoIterator for UsablePhysicalPageRanges<'a, M, P>
where
    M: MemoryMap<Address = P::Address> + ?Sized,
    P: PhysicalPage,
{
    type Item = PhysicalPageRange<P>;
    type IntoIter = UsablePhysicalPageRangeIter<'a, M, P>;

    fn into_iter(self) -> Self::IntoIter {
        UsablePhysicalPageRangeIter {
            regions: self.memory_map.iter(),
            arithmetic_context: self.arithmetic_context,
            page: PhantomData,
        }
    }
}

pub struct UsablePhysicalPageRangeIter<'a, M, P>
where
    M: MemoryMap<Address = P::Address> + ?Sized + 'a,
    P: PhysicalPage,
{
    regions: M::RegionIter<'a>,
    arithmetic_context: P::ArithmeticContext,
    page: PhantomData<fn() -> P>,
}

impl<'a, M, P> Clone for UsablePhysicalPageRangeIter<'a, M, P>
where
    M: MemoryMap<Address = P::Address> + ?Sized + 'a,
    P: PhysicalPage,
{
    fn clone(&self) -> Self {
        Self {
            regions: self.regions.clone(),
            arithmetic_context: self.arithmetic_context,
            page: PhantomData,
        }
    }
}

impl<M, P> Iterator for UsablePhysicalPageRangeIter<'_, M, P>
where
    M: MemoryMap<Address = P::Address> + ?Sized,
    P: PhysicalPage,
{
    type Item = PhysicalPageRange<P>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let region = self.regions.next()?.unwrap_or_else(|error| {
                panic!("validated memory map changed while iterating usable pages: {error:?}")
            });

            match complete_usable_pages(region, self.arithmetic_context) {
                Ok(Some(range)) => return Some(range),
                Ok(None) => {}
                Err(error) => {
                    panic!("validated usable page range changed while iterating: {error:?}")
                }
            }
        }
    }
}

fn complete_usable_pages<P>(
    region: PhysicalMemoryRegion<P::Address>,
    arithmetic_context: P::ArithmeticContext,
) -> Result<Option<PhysicalPageRange<P>>, UsablePageRangeError>
where
    P: PhysicalPage,
{
    if region.kind() != MemoryRegionKind::Usable {
        return Ok(None);
    }
    if P::SIZE == 0 {
        return Err(UsablePageRangeError::InvalidPageSize);
    }

    let region_start = region.start().value();
    let region_end = region_start
        .checked_add(region.byte_len())
        .ok_or(UsablePageRangeError::AddressOverflow)?;
    let containing_page = P::containing_address(region.start());
    let start = if containing_page.start_address().value() == region_start {
        containing_page
    } else {
        containing_page
            .checked_add(1, arithmetic_context)
            .ok_or(UsablePageRangeError::InvalidPageRange)?
    };
    let aligned_start = start.start_address().value();
    if aligned_start >= region_end {
        return Ok(None);
    }

    let page_count = (region_end - aligned_start) / P::SIZE;
    if page_count == 0 {
        return Ok(None);
    }
    let page_count =
        usize::try_from(page_count).map_err(|_| UsablePageRangeError::PageCountOverflow)?;
    let range = PhysicalPageRange::new(start, page_count, arithmetic_context)
        .ok_or(UsablePageRangeError::InvalidPageRange)?;

    Ok(Some(range))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        arch::common::paging::Page as _,
        arch::x86_64::paging::address::{PhysicalAddress, PhysicalPage},
        boot::protocol::memory::PhysicalMemoryRegion,
    };

    const MAX_PHYSICAL_ADDRESS_BITS: u8 = 52;

    fn address(value: u64) -> PhysicalAddress {
        PhysicalAddress::new(value, MAX_PHYSICAL_ADDRESS_BITS).unwrap()
    }

    fn region(
        start: u64,
        byte_len: u64,
        kind: MemoryRegionKind,
    ) -> PhysicalMemoryRegion<PhysicalAddress> {
        PhysicalMemoryRegion::new(address(start), byte_len, kind).unwrap()
    }

    #[test]
    fn retains_only_pages_fully_contained_in_a_usable_region() {
        let pages = complete_usable_pages::<PhysicalPage>(
            region(0x1003, 0x3000, MemoryRegionKind::Usable),
            MAX_PHYSICAL_ADDRESS_BITS,
        )
        .unwrap()
        .unwrap();

        assert_eq!(pages.start().start_address().value(), 0x2000);
        assert_eq!(pages.page_count(), 2);
    }

    #[test]
    fn ignores_nonusable_and_subpage_regions() {
        assert_eq!(
            complete_usable_pages::<PhysicalPage>(
                region(0x1000, 0x1000, MemoryRegionKind::Reserved),
                MAX_PHYSICAL_ADDRESS_BITS,
            ),
            Ok(None),
        );
        assert_eq!(
            complete_usable_pages::<PhysicalPage>(
                region(0x1001, 0x0ffe, MemoryRegionKind::Usable),
                MAX_PHYSICAL_ADDRESS_BITS,
            ),
            Ok(None),
        );
    }
}
