use core::ptr::NonNull;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RbColor {
    Red,
    Black,
}

#[derive(Debug)]
pub struct FreeBlockLinks {
    pub parent: Option<NonNull<FreeBlock>>,
    pub left: Option<NonNull<FreeBlock>>,
    pub right: Option<NonNull<FreeBlock>>,
    pub color: RbColor,
}

/// Intrusive metadata stored at the beginning of a mapped free heap block.
#[derive(Debug)]
pub struct FreeBlock {
    pub size: usize,
    pub address_links: FreeBlockLinks,
    pub size_links: FreeBlockLinks,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AllocationHeader {
    pub block_start: NonNull<u8>,
    pub block_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeBlockTreeIndex {
    Address,
    Size,
}

pub struct FreeBlockTree {
    pub root: Option<NonNull<FreeBlock>>,
    pub len: usize,
    pub index: FreeBlockTreeIndex,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl FreeBlockTree {
    pub fn new(index: FreeBlockTreeIndex) -> Self {
        todo!()
    }

    /// # Safety
    ///
    /// `block` must point to a live mapped free block and must not already be
    /// linked through this tree's selected link set.
    pub unsafe fn insert(&mut self, block: NonNull<FreeBlock>) {
        todo!()
    }

    /// # Safety
    ///
    /// `block` must be live, mapped, and currently linked into this tree.
    pub unsafe fn remove(&mut self, block: NonNull<FreeBlock>) {
        todo!()
    }

    pub fn find_suitable(&self, size: usize, alignment: usize) -> Option<NonNull<FreeBlock>> {
        todo!()
    }

    /// # Safety
    ///
    /// `block` must be live, mapped, and currently linked into this tree.
    pub unsafe fn predecessor(&self, block: NonNull<FreeBlock>) -> Option<NonNull<FreeBlock>> {
        todo!()
    }

    /// # Safety
    ///
    /// `block` must be live, mapped, and currently linked into this tree.
    pub unsafe fn successor(&self, block: NonNull<FreeBlock>) -> Option<NonNull<FreeBlock>> {
        todo!()
    }
}
