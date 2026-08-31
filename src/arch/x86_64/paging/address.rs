pub const PAGE_SIZE: u64 = 4096;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    pub const fn new(value: u64, max_physical_address_bits: u8) -> Option<Self> {
        match max_physical_address_bits {
            1..=63 if value < (1u64 << max_physical_address_bits) => Some(Self(value)),
            64 => Some(Self(value)),
            _ => None,
        }
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(u64);

impl VirtualAddress {
    pub const fn new(value: u64) -> Option<Self> {
        let upper = value >> 48;
        let expected_upper = if value & (1 << 47) == 0 {
            0x0000
        } else {
            0xffff
        };

        if upper == expected_upper {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalPage(u64);

impl PhysicalPage {
    pub fn from_start_address(address: PhysicalAddress) -> Option<Self> {
        if address.value() % PAGE_SIZE == 0 {
            Some(Self(address.value() / PAGE_SIZE))
        } else {
            None
        }
    }

    pub fn start_address(self) -> PhysicalAddress {
        // A PhysicalPage can only be constructed from a validated, page-aligned
        // PhysicalAddress or by checked_add, so this multiplication cannot exceed
        // the MAXPHYADDR limit used to construct it.
        PhysicalAddress(self.0 * PAGE_SIZE)
    }

    pub fn checked_add(self, page_count: usize, max_physical_address_bits: u8) -> Option<Self> {
        let page_count = u64::try_from(page_count).ok()?;
        let page_number = self.0.checked_add(page_count)?;
        let start_address = page_number.checked_mul(PAGE_SIZE)?;

        PhysicalAddress::new(start_address, max_physical_address_bits)?;
        Some(Self(page_number))
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualPage(u64);

impl VirtualPage {
    const TABLE_INDEX_MASK: u64 = 0b111111111;

    pub fn from_start_address(address: VirtualAddress) -> Option<Self> {
        if address.value() % PAGE_SIZE == 0 {
            Some(Self(address.value() / PAGE_SIZE))
        } else {
            None
        }
    }

    pub fn start_address(self) -> VirtualAddress {
        VirtualAddress(self.0 * PAGE_SIZE)
    }

    pub fn checked_add(self, page_count: usize) -> Option<Self> {
        let page_count = u64::try_from(page_count).ok()?;
        let page_number = self.0.checked_add(page_count)?;
        let start_address = page_number.checked_mul(PAGE_SIZE)?;

        VirtualAddress::new(start_address)?;
        Some(Self(page_number))
    }

    pub const fn level4_index(self) -> usize {
        ((self.0 >> 27) & Self::TABLE_INDEX_MASK) as usize
    }

    pub const fn level3_index(self) -> usize {
        ((self.0 >> 18) & Self::TABLE_INDEX_MASK) as usize
    }

    pub const fn level2_index(self) -> usize {
        ((self.0 >> 9) & Self::TABLE_INDEX_MASK) as usize
    }

    pub const fn level1_index(self) -> usize {
        (self.0 & Self::TABLE_INDEX_MASK) as usize
    }
}
