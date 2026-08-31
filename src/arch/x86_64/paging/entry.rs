use core::marker::PhantomData;

use crate::arch::common::paging::{Address as _, Page as _};

use super::{
    address::{PhysicalAddress, PhysicalPage},
    level::{Level1, Level2, Level3, Level4, TableLevel},
};

const ENTRY_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableEntry<L>
where
    L: TableLevel,
{
    bits: u64,
    level: PhantomData<L>,
}

impl<L> PageTableEntry<L>
where
    L: TableLevel,
{
    pub const fn empty() -> Self {
        Self {
            bits: 0,
            level: PhantomData,
        }
    }

    pub fn new(page: PhysicalPage, flags: PageFlags<L>) -> Self {
        Self {
            bits: page.start_address().value() | flags.bits(),
            level: PhantomData,
        }
    }

    pub fn physical_page(&self, max_physical_address_bits: u8) -> Option<PhysicalPage> {
        if !self.contains_flags(PageFlags::PRESENT) {
            return None;
        }

        let address =
            PhysicalAddress::new(self.bits & ENTRY_ADDRESS_MASK, max_physical_address_bits)?;
        PhysicalPage::from_start_address(address)
    }

    pub fn contains_flags(&self, flags: PageFlags<L>) -> bool {
        self.bits & flags.bits() == flags.bits()
    }

    pub fn matches_flags(&self, flags: PageFlags<L>) -> bool {
        self.flags() == flags
    }

    pub fn insert_flags(&mut self, flags: PageFlags<L>) {
        self.bits |= flags.bits();
    }

    pub fn remove_flags(&mut self, flags: PageFlags<L>) {
        self.bits &= !flags.bits();
    }

    pub fn flags(&self) -> PageFlags<L> {
        PageFlags::from_bits(self.bits & PageFlags::<L>::REPRESENTABLE_BITS)
            .expect("masked page-table flags must be representable")
    }

    pub fn clear(&mut self) {
        *self = Self::empty();
    }
}

impl<L> Default for PageTableEntry<L>
where
    L: TableLevel,
{
    fn default() -> Self {
        Self::empty()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFlags<L>
where
    L: TableLevel,
{
    bits: u64,
    level: PhantomData<L>,
}

impl<L> PageFlags<L>
where
    L: TableLevel,
{
    const REPRESENTABLE_BITS: u64 = 0x80_00_00_00_00_00_0f_ff;

    pub const NONE: Self = Self::from_bits(0).expect("NONE must be valid");
    pub const PRESENT: Self = Self::from_bits(1 << 0).expect("PRESENT must be valid");
    pub const WRITABLE: Self = Self::from_bits(1 << 1).expect("WRITABLE must be valid");
    pub const USER: Self = Self::from_bits(1 << 2).expect("USER must be valid");
    pub const NO_EXECUTE: Self = Self::from_bits(1 << 63).expect("NO_EXECUTE must be valid");

    pub const fn from_bits(bits: u64) -> Option<Self> {
        if bits & !Self::REPRESENTABLE_BITS == 0 {
            Some(Self {
                bits,
                level: PhantomData,
            })
        } else {
            None
        }
    }

    pub const fn bits(self) -> u64 {
        self.bits
    }

    pub const fn contains(self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }
}

impl PageFlags<Level3> {
    pub const HUGE_PAGE: Self = Self::from_bits(1 << 7).expect("HUGE_PAGE must be valid");
}

impl PageFlags<Level2> {
    pub const HUGE_PAGE: Self = Self::from_bits(1 << 7).expect("HUGE_PAGE must be valid");
}

impl<L> core::ops::BitOr for PageFlags<L>
where
    L: TableLevel,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::from_bits(self.bits | rhs.bits).expect("combining valid page flags must remain valid")
    }
}

impl<L> core::ops::BitOrAssign for PageFlags<L>
where
    L: TableLevel,
{
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

pub type Pml4Entry = PageTableEntry<Level4>;
pub type PdptEntry = PageTableEntry<Level3>;
pub type PdEntry = PageTableEntry<Level2>;
pub type PtEntry = PageTableEntry<Level1>;

pub type Pml4Flags = PageFlags<Level4>;
pub type PdptFlags = PageFlags<Level3>;
pub type PdFlags = PageFlags<Level2>;
pub type PtFlags = PageFlags<Level1>;

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_PHYSICAL_ADDRESS_BITS: u8 = 52;

    fn physical_page(address: u64) -> PhysicalPage {
        let address = PhysicalAddress::new(address, MAX_PHYSICAL_ADDRESS_BITS).unwrap();
        PhysicalPage::from_start_address(address).unwrap()
    }

    #[test]
    fn creates_an_entry_and_extracts_its_page_and_flags() {
        let page = physical_page(0x1234_5000);
        let flags = PtFlags::PRESENT | PtFlags::WRITABLE | PtFlags::NO_EXECUTE;
        let entry = PtEntry::new(page, flags);

        assert_eq!(entry.physical_page(MAX_PHYSICAL_ADDRESS_BITS), Some(page));
        assert_eq!(entry.flags(), flags);
        assert!(entry.contains_flags(PtFlags::PRESENT));
        assert!(entry.contains_flags(PtFlags::PRESENT | PtFlags::WRITABLE));
        assert!(entry.matches_flags(flags));
        assert!(!entry.matches_flags(PtFlags::PRESENT));
    }

    #[test]
    fn changes_flags_without_changing_the_physical_page() {
        let page = physical_page(0x2345_6000);
        let mut entry = PtEntry::new(page, PtFlags::PRESENT);

        entry.insert_flags(PtFlags::WRITABLE | PtFlags::USER);
        entry.insert_flags(PtFlags::NO_EXECUTE);
        entry.remove_flags(PtFlags::WRITABLE);

        assert_eq!(entry.physical_page(MAX_PHYSICAL_ADDRESS_BITS), Some(page));
        assert!(!entry.contains_flags(PtFlags::WRITABLE));
        assert!(entry.contains_flags(PtFlags::USER));
        assert!(entry.contains_flags(PtFlags::NO_EXECUTE));
    }

    #[test]
    fn clear_removes_the_address_and_all_flags() {
        let page = physical_page(0x3456_7000);
        let mut entry = PtEntry::new(page, PtFlags::PRESENT | PtFlags::USER);

        entry.clear();

        assert_eq!(entry, PtEntry::empty());
        assert_eq!(entry.physical_page(MAX_PHYSICAL_ADDRESS_BITS), None);
        assert_eq!(entry.flags(), PtFlags::NONE);
    }

    #[test]
    fn rejects_bits_outside_the_representable_flag_fields() {
        assert!(PtFlags::from_bits(1 << 12).is_none());
        assert!(PtFlags::from_bits(1 << 62).is_none());
    }

    #[test]
    fn recognizes_huge_page_flags_at_supported_levels() {
        let page = physical_page(0x4000_0000);
        let level3 = PdptEntry::new(page, PdptFlags::PRESENT | PdptFlags::HUGE_PAGE);
        let level2 = PdEntry::new(page, PdFlags::PRESENT | PdFlags::HUGE_PAGE);

        assert!(level3.contains_flags(PdptFlags::HUGE_PAGE));
        assert!(level2.contains_flags(PdFlags::HUGE_PAGE));
    }
}
