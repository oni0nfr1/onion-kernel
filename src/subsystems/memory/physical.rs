use core::{
    mem::{align_of, size_of},
    ptr, slice,
};

use crate::{
    arch::common::paging::{
        Address as _, DirectMap, PageTablePageProvider, PhysicalPage, PhysicalPageRange,
    },
    util::bitmap::Bitmap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalRegionDescriptor<P>
where
    P: PhysicalPage,
{
    start_page: P,
    page_count: usize,
    bitmap_offset: usize,
    bitmap_byte_count: usize,
    next_hint: usize,
}

impl<P> PhysicalRegionDescriptor<P>
where
    P: PhysicalPage,
{
    fn from_region(region: PhysicalPageRange<P>, bitmap_offset: usize) -> Self {
        Self {
            start_page: region.start(),
            page_count: region.page_count(),
            bitmap_offset,
            bitmap_byte_count: bitmap_bytes_for_pages(region.page_count()),
            next_hint: 0,
        }
    }

    pub fn start_page(&self) -> P {
        self.start_page
    }

    pub fn page_count(&self) -> usize {
        self.page_count
    }

    pub fn bitmap_byte_count(&self) -> usize {
        self.bitmap_byte_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalAllocatorInitError {
    NoUsablePages,
    NoMetadataRegion,
    InvalidRegion,
    OverlappingRegions,
    MetadataUnavailable,
    AddressOverflow,
    SizeOverflow,
}

/// Allocates individual physical pages from independent bitmap-backed regions.
///
/// Descriptor and bitmap storage are identified by physical pages rather than
/// retained virtual references. This lets the allocator preserve its state
/// while changing the DirectMap used to access that storage.
pub struct BitmapPageAllocator<P, D>
where
    P: PhysicalPage,
    D: DirectMap<PhysicalAddress = P::Address>,
{
    region_storage_start: P,
    region_count: usize,
    bitmap_storage_start: P,
    bitmap_byte_count: usize,
    metadata_page_count: usize,
    next_region_hint: usize,
    arithmetic_context: P::ArithmeticContext,
    direct_map: D,
}

impl<P, D> BitmapPageAllocator<P, D>
where
    P: PhysicalPage,
    D: DirectMap<PhysicalAddress = P::Address>,
{
    /// Bootstraps an allocator by reserving metadata from `usable_regions`.
    ///
    /// Descriptor and bitmap pages are taken from the beginning of one usable
    /// region and excluded from the resulting free-page descriptors.
    ///
    /// # Safety
    ///
    /// Every input region must represent exclusively available physical
    /// memory. `direct_map` must provide stable writable access to the complete
    /// metadata range chosen from those regions. No other owner may access or
    /// allocate the metadata pages while the returned allocator exists. Every
    /// clone of `usable_regions` must yield the same finite sequence of
    /// regions.
    pub unsafe fn bootstrap<I>(
        usable_regions: I,
        arithmetic_context: P::ArithmeticContext,
        direct_map: D,
    ) -> Result<Self, PhysicalAllocatorInitError>
    where
        I: Clone + IntoIterator<Item = PhysicalPageRange<P>>,
    {
        let input_region_count = validate_regions(&usable_regions, arithmetic_context)?;
        let total_input_pages =
            usable_regions
                .clone()
                .into_iter()
                .try_fold(0usize, |total, region| {
                    total
                        .checked_add(region.page_count())
                        .ok_or(PhysicalAllocatorInitError::SizeOverflow)
                })?;
        let descriptor_bytes = input_region_count
            .checked_mul(size_of::<PhysicalRegionDescriptor<P>>())
            .ok_or(PhysicalAllocatorInitError::SizeOverflow)?;
        let descriptor_page_count = pages_for_bytes::<P>(descriptor_bytes)?;
        // Each region owns a byte-aligned bitmap, so summing the regions'
        // individually rounded sizes is required. Rounding only the combined
        // page count could underestimate storage for many small regions.
        let conservative_bitmap_bytes =
            usable_regions
                .clone()
                .into_iter()
                .try_fold(0usize, |total, region| {
                    total
                        .checked_add(bitmap_bytes_for_pages(region.page_count()))
                        .ok_or(PhysicalAllocatorInitError::SizeOverflow)
                })?;
        let bitmap_page_count = pages_for_bytes::<P>(conservative_bitmap_bytes)?;
        let metadata_page_count = descriptor_page_count
            .checked_add(bitmap_page_count)
            .ok_or(PhysicalAllocatorInitError::SizeOverflow)?;

        if total_input_pages <= metadata_page_count {
            return Err(PhysicalAllocatorInitError::NoUsablePages);
        }

        let (metadata_region_index, metadata_region) = usable_regions
            .clone()
            .into_iter()
            .enumerate()
            .find(|(_, region)| region.page_count() >= metadata_page_count)
            .ok_or(PhysicalAllocatorInitError::NoMetadataRegion)?;
        let region_storage_start = metadata_region.start();
        let bitmap_storage_start = region_storage_start
            .checked_add(descriptor_page_count, arithmetic_context)
            .ok_or(PhysicalAllocatorInitError::AddressOverflow)?;

        let descriptor_address = mapped_storage_address(
            &direct_map,
            region_storage_start,
            align_of::<PhysicalRegionDescriptor<P>>(),
        )
        .ok_or(PhysicalAllocatorInitError::MetadataUnavailable)?;
        let bitmap_address =
            mapped_storage_address(&direct_map, bitmap_storage_start, align_of::<u8>())
                .ok_or(PhysicalAllocatorInitError::MetadataUnavailable)?;

        let mut region_count = 0usize;
        let mut bitmap_byte_count = 0usize;
        let mut metadata_region_seen = false;
        for (index, region) in usable_regions.into_iter().enumerate() {
            if region_count >= input_region_count {
                return Err(PhysicalAllocatorInitError::InvalidRegion);
            }

            let region = if index == metadata_region_index {
                metadata_region_seen = true;
                if region.page_count() == metadata_page_count {
                    continue;
                }
                let start = region
                    .start()
                    .checked_add(metadata_page_count, arithmetic_context)
                    .ok_or(PhysicalAllocatorInitError::AddressOverflow)?;
                let page_count = region.page_count() - metadata_page_count;
                PhysicalPageRange::new(start, page_count, arithmetic_context)
                    .ok_or(PhysicalAllocatorInitError::InvalidRegion)?
            } else {
                region
            };

            let descriptor = PhysicalRegionDescriptor::from_region(region, bitmap_byte_count);
            bitmap_byte_count = bitmap_byte_count
                .checked_add(descriptor.bitmap_byte_count)
                .ok_or(PhysicalAllocatorInitError::SizeOverflow)?;

            // SAFETY: The bootstrap contract grants exclusive writable access
            // to the selected descriptor pages, and capacity was calculated
            // for every input region.
            unsafe {
                (descriptor_address as *mut PhysicalRegionDescriptor<P>)
                    .add(region_count)
                    .write(descriptor);
            }
            region_count += 1;
        }

        if !metadata_region_seen {
            return Err(PhysicalAllocatorInitError::InvalidRegion);
        }

        let bitmap_capacity = bitmap_page_count
            .checked_mul(page_size::<P>()?)
            .ok_or(PhysicalAllocatorInitError::SizeOverflow)?;
        if bitmap_byte_count > bitmap_capacity {
            return Err(PhysicalAllocatorInitError::SizeOverflow);
        }
        // SAFETY: The selected bitmap pages are exclusively writable through
        // `direct_map`, and `bitmap_capacity` is exactly their byte extent.
        unsafe { ptr::write_bytes(bitmap_address as *mut u8, 0, bitmap_capacity) };

        Ok(Self {
            region_storage_start,
            region_count,
            bitmap_storage_start,
            bitmap_byte_count,
            metadata_page_count,
            next_region_hint: 0,
            arithmetic_context,
            direct_map,
        })
    }

    pub fn direct_map(&self) -> &D {
        &self.direct_map
    }

    pub fn metadata_region(&self) -> PhysicalPageRange<P> {
        PhysicalPageRange::new(
            self.region_storage_start,
            self.metadata_page_count,
            self.arithmetic_context,
        )
        .expect("allocator metadata must remain a valid non-empty physical page range")
    }

    /// Rebinds the allocator's metadata access to another DirectMap without
    /// changing any allocation bit.
    ///
    /// # Safety
    ///
    /// `direct_map` must map the complete descriptor and bitmap storage ranges
    /// at stable, writable addresses. No reference obtained through the old
    /// DirectMap may remain in use after this call.
    pub unsafe fn reconstruct_with_direct_map<N>(self, direct_map: N) -> BitmapPageAllocator<P, N>
    where
        N: DirectMap<PhysicalAddress = P::Address>,
    {
        assert_storage_accessible(
            &direct_map,
            self.region_storage_start,
            align_of::<PhysicalRegionDescriptor<P>>(),
            "physical allocator descriptor storage",
        );
        assert_storage_accessible(
            &direct_map,
            self.bitmap_storage_start,
            align_of::<u8>(),
            "physical allocator bitmap storage",
        );

        BitmapPageAllocator {
            region_storage_start: self.region_storage_start,
            region_count: self.region_count,
            bitmap_storage_start: self.bitmap_storage_start,
            bitmap_byte_count: self.bitmap_byte_count,
            metadata_page_count: self.metadata_page_count,
            next_region_hint: self.next_region_hint,
            arithmetic_context: self.arithmetic_context,
            direct_map,
        }
    }

    pub fn allocate(&mut self) -> Option<P> {
        for offset in 0..self.region_count {
            let remaining_after_hint = self.region_count - self.next_region_hint;
            let descriptor_index = if offset < remaining_after_hint {
                self.next_region_hint + offset
            } else {
                offset - remaining_after_hint
            };
            let descriptor = *self.regions().get(descriptor_index)?;
            let next_hint = descriptor.next_hint;

            let bit_index = {
                let mut region = self.region_bitmap(descriptor_index)?;
                let Some(bit_index) = region.bitmap.find_next(next_hint, false) else {
                    continue;
                };
                region.bitmap.set(bit_index, true)?;
                bit_index
            };

            let page = descriptor
                .start_page
                .checked_add(bit_index, self.arithmetic_context);
            let Some(page) = page else {
                let mut region = self.region_bitmap(descriptor_index)?;
                region.bitmap.set(bit_index, false)?;
                continue;
            };

            let region_count = self.region_count;
            let descriptor = self.regions_mut().get_mut(descriptor_index)?;
            descriptor.next_hint = bit_index.checked_add(1).unwrap_or(0);
            self.next_region_hint = (descriptor_index + 1) % region_count;
            return Some(page);
        }

        None
    }

    /// # Safety
    ///
    /// `page` must have been returned by this allocator, must still be
    /// allocated, and must no longer be used or referenced anywhere.
    pub unsafe fn deallocate(&mut self, page: P) {
        let page_address = page.start_address().value();
        let descriptor_index = self
            .regions()
            .iter()
            .position(|descriptor| {
                let start = descriptor.start_page.start_address().value();
                let last = descriptor
                    .start_page
                    .checked_add(descriptor.page_count - 1, self.arithmetic_context)
                    .expect("validated physical region must retain a valid last page")
                    .start_address()
                    .value();
                start <= page_address && page_address <= last
            })
            .unwrap_or_else(|| panic!("attempted to deallocate unmanaged physical page {page:?}"));

        let descriptor = self.regions()[descriptor_index];
        let byte_offset = page_address - descriptor.start_page.start_address().value();
        assert!(
            byte_offset.is_multiple_of(P::SIZE),
            "physical page {page:?} is not aligned relative to managed region {:?}",
            descriptor.start_page,
        );
        let bit_index = usize::try_from(byte_offset / P::SIZE)
            .expect("managed physical page offset must fit usize");

        {
            let mut region = self
                .region_bitmap(descriptor_index)
                .expect("validated physical region must have bitmap storage");
            assert_eq!(
                region.bitmap.get(bit_index),
                Some(true),
                "attempted to deallocate free physical page {page:?}",
            );
            let result = region.bitmap.set(bit_index, false);
            assert!(
                result.is_some(),
                "managed physical page bit must be inside its bitmap"
            );
        }

        let descriptor = &mut self.regions_mut()[descriptor_index];
        descriptor.next_hint = descriptor.next_hint.min(bit_index);
        self.next_region_hint = descriptor_index;
    }

    fn regions(&self) -> &[PhysicalRegionDescriptor<P>] {
        let address = mapped_storage_address(
            &self.direct_map,
            self.region_storage_start,
            align_of::<PhysicalRegionDescriptor<P>>(),
        )
        .expect("physical allocator descriptor storage must remain DirectMap-accessible");

        // SAFETY: `bootstrap` reserves this complete storage exclusively.
        // Reconstruction requires the new DirectMap to preserve it.
        unsafe {
            slice::from_raw_parts(
                address as *const PhysicalRegionDescriptor<P>,
                self.region_count,
            )
        }
    }

    fn regions_mut(&mut self) -> &mut [PhysicalRegionDescriptor<P>] {
        let address = mapped_storage_address(
            &self.direct_map,
            self.region_storage_start,
            align_of::<PhysicalRegionDescriptor<P>>(),
        )
        .expect("physical allocator descriptor storage must remain DirectMap-accessible");

        // SAFETY: The allocator owns the original exclusive storage borrow, and
        // `&mut self` prevents another live mutable view from being created.
        unsafe {
            slice::from_raw_parts_mut(
                address as *mut PhysicalRegionDescriptor<P>,
                self.region_count,
            )
        }
    }

    fn bitmap_storage_mut(&mut self) -> &mut [u8] {
        let address = mapped_storage_address(
            &self.direct_map,
            self.bitmap_storage_start,
            align_of::<u8>(),
        )
        .expect("physical allocator bitmap storage must remain DirectMap-accessible");

        // SAFETY: Same exclusive-storage argument as `regions_mut`.
        unsafe { slice::from_raw_parts_mut(address as *mut u8, self.bitmap_byte_count) }
    }

    fn region_bitmap(&mut self, descriptor_index: usize) -> Option<RegionBitmap<'_>> {
        let descriptor = *self.regions().get(descriptor_index)?;
        let end = descriptor
            .bitmap_offset
            .checked_add(descriptor.bitmap_byte_count)?;
        let storage = self
            .bitmap_storage_mut()
            .get_mut(descriptor.bitmap_offset..end)?;
        let bitmap = Bitmap::new(storage, descriptor.page_count)?;

        Some(RegionBitmap { bitmap })
    }
}

struct RegionBitmap<'a> {
    bitmap: Bitmap<'a>,
}

impl<P, D> PageTablePageProvider for BitmapPageAllocator<P, D>
where
    P: PhysicalPage,
    D: DirectMap<PhysicalAddress = P::Address>,
{
    type PhysicalPage = P;

    fn allocate_page_table_page(&mut self) -> Option<P> {
        self.allocate()
    }

    unsafe fn release_page_table_page(&mut self, page: P) {
        // SAFETY: The caller must uphold `PageTablePageProvider`'s release
        // contract, which is at least as strict as `deallocate`'s contract.
        unsafe { self.deallocate(page) };
    }
}

fn bitmap_bytes_for_pages(page_count: usize) -> usize {
    page_count / u8::BITS as usize + usize::from(!page_count.is_multiple_of(u8::BITS as usize))
}

fn validate_regions<P, I>(
    regions: &I,
    arithmetic_context: P::ArithmeticContext,
) -> Result<usize, PhysicalAllocatorInitError>
where
    P: PhysicalPage,
    I: Clone + IntoIterator<Item = PhysicalPageRange<P>>,
{
    let mut region_count = 0usize;
    for (index, region) in regions.clone().into_iter().enumerate() {
        let region_last = region
            .last(arithmetic_context)
            .ok_or(PhysicalAllocatorInitError::AddressOverflow)?;

        for other in regions.clone().into_iter().take(index) {
            let other_last = other
                .last(arithmetic_context)
                .ok_or(PhysicalAllocatorInitError::AddressOverflow)?;
            if region.start() <= other_last && other.start() <= region_last {
                return Err(PhysicalAllocatorInitError::OverlappingRegions);
            }
        }
        region_count = region_count
            .checked_add(1)
            .ok_or(PhysicalAllocatorInitError::SizeOverflow)?;
    }

    if region_count == 0 {
        return Err(PhysicalAllocatorInitError::NoUsablePages);
    }

    Ok(region_count)
}

fn page_size<P>() -> Result<usize, PhysicalAllocatorInitError>
where
    P: PhysicalPage,
{
    usize::try_from(P::SIZE).map_err(|_| PhysicalAllocatorInitError::SizeOverflow)
}

fn pages_for_bytes<P>(byte_count: usize) -> Result<usize, PhysicalAllocatorInitError>
where
    P: PhysicalPage,
{
    let page_size = page_size::<P>()?;
    if page_size == 0 {
        return Err(PhysicalAllocatorInitError::InvalidRegion);
    }

    Ok(byte_count / page_size + usize::from(!byte_count.is_multiple_of(page_size)))
}

fn mapped_storage_address<P, D>(direct_map: &D, start: P, alignment: usize) -> Option<usize>
where
    P: PhysicalPage,
    D: DirectMap<PhysicalAddress = P::Address>,
{
    let address = direct_map
        .physical_to_virtual(start.start_address())?
        .value();
    let address = usize::try_from(address).ok()?;
    address.is_multiple_of(alignment).then_some(address)
}

fn assert_storage_accessible<P, D>(direct_map: &D, start: P, alignment: usize, name: &str)
where
    P: PhysicalPage,
    D: DirectMap<PhysicalAddress = P::Address>,
{
    assert!(
        mapped_storage_address(direct_map, start, alignment).is_some(),
        "{name} must be accessible and properly aligned through the new DirectMap",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::arch::common::paging::PhysicalPage as _;
    use crate::arch::x86_64::paging::address::{PhysicalAddress, PhysicalPage, VirtualAddress};

    const MAX_PHYSICAL_ADDRESS_BITS: u8 = 52;

    #[derive(Debug, Clone, Copy)]
    struct IdentityDirectMap;

    impl DirectMap for IdentityDirectMap {
        type PhysicalAddress = PhysicalAddress;
        type VirtualAddress = VirtualAddress;

        fn physical_to_virtual(
            &self,
            address: Self::PhysicalAddress,
        ) -> Option<Self::VirtualAddress> {
            VirtualAddress::new(address.value())
        }

        fn virtual_to_physical(
            &self,
            address: Self::VirtualAddress,
        ) -> Option<Self::PhysicalAddress> {
            PhysicalAddress::new(address.value(), MAX_PHYSICAL_ADDRESS_BITS)
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct ReconstructedIdentityDirectMap;

    impl DirectMap for ReconstructedIdentityDirectMap {
        type PhysicalAddress = PhysicalAddress;
        type VirtualAddress = VirtualAddress;

        fn physical_to_virtual(
            &self,
            address: Self::PhysicalAddress,
        ) -> Option<Self::VirtualAddress> {
            VirtualAddress::new(address.value())
        }

        fn virtual_to_physical(
            &self,
            address: Self::VirtualAddress,
        ) -> Option<Self::PhysicalAddress> {
            PhysicalAddress::new(address.value(), MAX_PHYSICAL_ADDRESS_BITS)
        }
    }

    #[repr(C, align(4096))]
    struct LargeUsableStorage([u8; 8 * 4096]);

    #[repr(C, align(4096))]
    struct SmallUsableStorage([u8; 4 * 4096]);

    fn page(address: u64) -> PhysicalPage {
        let address = PhysicalAddress::new(address, MAX_PHYSICAL_ADDRESS_BITS).unwrap();
        PhysicalPage::from_start_address(address).unwrap()
    }

    fn storage_page<T>(storage: &mut T) -> PhysicalPage {
        page(storage as *mut T as usize as u64)
    }

    fn region(start_page: PhysicalPage, page_count: usize) -> PhysicalPageRange<PhysicalPage> {
        PhysicalPageRange::new(start_page, page_count, MAX_PHYSICAL_ADDRESS_BITS).unwrap()
    }

    #[test]
    fn reserves_metadata_and_allocates_only_the_remaining_pages() {
        let mut first_storage = LargeUsableStorage([0; 8 * 4096]);
        let mut second_storage = SmallUsableStorage([0; 4 * 4096]);
        let first_start = storage_page(&mut first_storage);
        let second_start = storage_page(&mut second_storage);
        let regions = [region(first_start, 8), region(second_start, 4)];

        // SAFETY: Both aligned test buffers are exclusively owned and exposed
        // by the identity DirectMap for the allocator's lifetime.
        let mut allocator = unsafe {
            BitmapPageAllocator::bootstrap(regions, MAX_PHYSICAL_ADDRESS_BITS, IdentityDirectMap)
        }
        .unwrap();
        let metadata = allocator.metadata_region();
        assert_eq!(metadata.start(), first_start);
        assert_eq!(metadata.page_count(), 2);

        let first = allocator.allocate().unwrap();
        let second = allocator.allocate().unwrap();
        assert_eq!(
            first,
            first_start
                .checked_add(metadata.page_count(), MAX_PHYSICAL_ADDRESS_BITS)
                .unwrap(),
        );
        assert_eq!(second, second_start);

        let mut allocation_count = 2;
        while allocator.allocate().is_some() {
            allocation_count += 1;
        }
        assert_eq!(allocation_count, 10);

        // SAFETY: `first` was allocated above and has no other users.
        unsafe { allocator.deallocate(first) };
        assert_eq!(allocator.allocate(), Some(first));
    }

    #[test]
    fn preserves_allocation_state_when_reconstructing_the_direct_map() {
        let mut storage = LargeUsableStorage([0; 8 * 4096]);
        let start = storage_page(&mut storage);
        let regions = [region(start, 8)];
        // SAFETY: The aligned buffer is exclusively owned and identity-mapped.
        let mut allocator = unsafe {
            BitmapPageAllocator::bootstrap(regions, MAX_PHYSICAL_ADDRESS_BITS, IdentityDirectMap)
        }
        .unwrap();

        let first = allocator.allocate().unwrap();
        // SAFETY: The replacement DirectMap exposes the same exclusively owned
        // metadata storage at stable writable addresses in this test.
        let mut allocator =
            unsafe { allocator.reconstruct_with_direct_map(ReconstructedIdentityDirectMap) };

        assert_ne!(allocator.allocate(), Some(first));
        // SAFETY: `first` remains allocated and has no users.
        unsafe { allocator.deallocate(first) };
        assert_eq!(allocator.allocate(), Some(first));
    }

    #[test]
    fn rejects_overlapping_managed_regions() {
        let regions = [region(page(0x40_0000), 2), region(page(0x40_1000), 2)];

        // SAFETY: Bootstrap rejects the overlap before selecting or accessing
        // any metadata storage.
        let result = unsafe {
            BitmapPageAllocator::bootstrap(regions, MAX_PHYSICAL_ADDRESS_BITS, IdentityDirectMap)
        };
        assert!(matches!(
            result,
            Err(PhysicalAllocatorInitError::OverlappingRegions)
        ));
    }

    #[test]
    fn rejects_regions_that_cannot_hold_contiguous_metadata() {
        let regions = [
            region(page(0x50_0000), 1),
            region(page(0x60_0000), 1),
            region(page(0x70_0000), 1),
        ];

        // SAFETY: Bootstrap fails before dereferencing the synthetic regions.
        let result = unsafe {
            BitmapPageAllocator::bootstrap(regions, MAX_PHYSICAL_ADDRESS_BITS, IdentityDirectMap)
        };
        assert!(matches!(
            result,
            Err(PhysicalAllocatorInitError::NoMetadataRegion)
        ));
    }
}
