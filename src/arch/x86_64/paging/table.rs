use core::ptr::NonNull;

use crate::arch::x86_64::paging::{entry::PageFlags, level::NonLeafTableLevel};

use super::{
    entry::PageTableEntry,
    level::{Level1, Level2, Level3, Level4, TableLevel},
};

pub const PAGE_TABLE_ENTRY_COUNT: usize = 512;

#[repr(C, align(4096))]
pub struct PageTable<L>
where
    L: TableLevel,
{
    entries: [PageTableEntry<L>; PAGE_TABLE_ENTRY_COUNT],
}

impl<L> PageTable<L>
where
    L: TableLevel,
{
    pub fn new() -> Self {
        Self {
            entries: [PageTableEntry::empty(); PAGE_TABLE_ENTRY_COUNT],
        }
    }

    pub fn entry(&self, index: usize) -> Option<&PageTableEntry<L>> {
        self.entries.get(index)
    }

    pub fn entry_mut(&mut self, index: usize) -> Option<&mut PageTableEntry<L>> {
        self.entries.get_mut(index)
    }

    pub fn clear(&mut self) {
        self.entries = [PageTableEntry::empty(); PAGE_TABLE_ENTRY_COUNT];
    }

    pub fn is_empty(&self) -> bool {
        self.entries
            .iter()
            .all(|entry| !entry.contains_flags(PageFlags::PRESENT))
    }
}

pub type Pml4 = PageTable<Level4>;
pub type Pdpt = PageTable<Level3>;
pub type Pd = PageTable<Level2>;
pub type Pt = PageTable<Level1>;

const _: () = assert!(size_of::<Pml4>() == 4096);
const _: () = assert!(size_of::<Pdpt>() == 4096);
const _: () = assert!(size_of::<Pd>() == 4096);
const _: () = assert!(size_of::<Pt>() == 4096);

const _: () = assert!(align_of::<Pml4>() == 4096);
const _: () = assert!(align_of::<Pdpt>() == 4096);
const _: () = assert!(align_of::<Pd>() == 4096);
const _: () = assert!(align_of::<Pt>() == 4096);
