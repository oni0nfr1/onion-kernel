use core::arch::asm;

use super::address::{PhysicalAddress, PhysicalPage};

const CR3_LOWER_BITS_MASK: u64 = 0x0fff;

/// A decoded value of the x86_64 CR3 register.
///
/// The interpretation of the lower 12 bits depends on whether CR4.PCIDE is
/// enabled. They are therefore preserved without interpretation here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cr3 {
    root_page: PhysicalPage,
    lower_bits: u16,
}

impl Cr3 {
    pub const fn new(root_page: PhysicalPage, lower_bits: u16) -> Option<Self> {
        if lower_bits as u64 & !CR3_LOWER_BITS_MASK == 0 {
            Some(Self {
                root_page,
                lower_bits,
            })
        } else {
            None
        }
    }

    pub const fn root_page(self) -> PhysicalPage {
        self.root_page
    }

    pub const fn lower_bits(self) -> u16 {
        self.lower_bits
    }

    fn from_raw(raw: u64, max_physical_address_bits: u8) -> Option<Self> {
        let root_address =
            PhysicalAddress::new(raw & !CR3_LOWER_BITS_MASK, max_physical_address_bits)?;
        let root_page = PhysicalPage::from_start_address(root_address)?;

        Self::new(root_page, (raw & CR3_LOWER_BITS_MASK) as u16)
    }

    fn raw(self) -> u64 {
        self.root_page.start_address().value() | u64::from(self.lower_bits)
    }
}

/// Reads and decodes the current CR3 value.
///
/// Returns `None` if the supplied physical-address width is invalid or the
/// register's page-table address does not fit within it.
#[inline]
pub fn read_cr3(max_physical_address_bits: u8) -> Option<Cr3> {
    let raw: u64;

    unsafe {
        asm!(
            "mov {}, cr3",
            out(reg) raw,
            options(nomem, nostack, preserves_flags),
        );
    }

    Cr3::from_raw(raw, max_physical_address_bits)
}

/// Writes a new value to CR3 and activates its Level4 page table.
///
/// This implementation writes bit 63 as zero and therefore does not request
/// the PCID no-flush behavior.
///
/// # Safety
///
/// The caller must ensure that `cr3.root_page()` contains a valid Level4 page
/// table and remains allocated while active. Its lower bits must be valid for
/// the current CR4.PCIDE setting. The new table must map all memory required to
/// continue execution, including the current instruction stream, stack, and
/// kernel data. Any required synchronization with other CPUs is also the
/// caller's responsibility.
#[inline]
pub unsafe fn write_cr3(cr3: Cr3) {
    unsafe {
        asm!(
            "mov cr3, {}",
            in(reg) cr3.raw(),
            options(nostack, preserves_flags),
        );
    }
}
