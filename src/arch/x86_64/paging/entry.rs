use core::marker::PhantomData;

use super::level::{Level1, Level2, Level3, Level4, TableLevel};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableEntry<L>
where
    L: TableLevel,
{
    bits: u64,
    level: PhantomData<L>,
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
    pub const EMPTY: Self = Self::from_bits(0);
    pub const PRESENT: Self = Self::from_bits(1 << 0);
    pub const WRITABLE: Self = Self::from_bits(1 << 1);
    pub const USER: Self = Self::from_bits(1 << 2);
    pub const NO_EXECUTE: Self = Self::from_bits(1 << 63);

    pub const fn from_bits(bits: u64) -> Self {
        Self {
            bits,
            level: PhantomData,
        }
    }

    pub const fn bits(self) -> u64 {
        self.bits
    }

    pub const fn contains(self, other: Self) -> bool {
        self.bits & other.bits == other.bits
    }
}

impl<L> core::ops::BitOr for PageFlags<L>
where
    L: TableLevel,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::from_bits(self.bits | rhs.bits)
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
