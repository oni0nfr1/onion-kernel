use core::arch::asm;

use super::address::VirtualAddress;

#[inline]
pub(crate) fn invalidate_page(address: VirtualAddress) {
    unsafe {
        asm!(
            "invlpg [{}]",
            in(reg) address.value(),
            options(nostack, preserves_flags),
        );
    }
}
