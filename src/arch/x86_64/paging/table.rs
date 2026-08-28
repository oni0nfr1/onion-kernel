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

#[expect(unused_variables, reason = "implementation scaffold")]
impl<L> PageTable<L>
where
    L: TableLevel,
{
    pub fn new() -> Self {
        todo!()
    }

    pub fn entry(&self, index: usize) -> Option<&PageTableEntry<L>> {
        todo!()
    }

    pub fn entry_mut(&mut self, index: usize) -> Option<&mut PageTableEntry<L>> {
        todo!()
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
