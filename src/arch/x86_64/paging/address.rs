pub const PAGE_SIZE: u64 = 4096;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(u64);

#[expect(unused_variables, reason = "implementation scaffold")]
impl PhysicalAddress {
    pub fn new(value: u64) -> Self {
        todo!()
    }

    pub fn value(self) -> u64 {
        todo!()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(u64);

#[expect(unused_variables, reason = "implementation scaffold")]
impl VirtualAddress {
    pub fn new(value: u64) -> Option<Self> {
        todo!()
    }

    pub fn value(self) -> u64 {
        todo!()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalFrame(u64);

#[expect(unused_variables, reason = "implementation scaffold")]
impl PhysicalFrame {
    pub fn from_start_address(address: PhysicalAddress) -> Option<Self> {
        todo!()
    }

    pub fn start_address(self) -> PhysicalAddress {
        todo!()
    }

    pub fn checked_add(self, frame_count: usize) -> Option<Self> {
        todo!()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualPage(u64);

#[expect(unused_variables, reason = "implementation scaffold")]
impl VirtualPage {
    pub fn from_start_address(address: VirtualAddress) -> Option<Self> {
        todo!()
    }

    pub fn start_address(self) -> VirtualAddress {
        todo!()
    }

    pub fn checked_add(self, page_count: usize) -> Option<Self> {
        todo!()
    }
}
