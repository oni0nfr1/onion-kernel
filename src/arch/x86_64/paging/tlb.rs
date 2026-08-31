#[cfg(not(test))]
use core::arch::asm;

#[cfg(not(test))]
use crate::arch::common::paging::Address as _;

use super::address::VirtualAddress;

#[inline]
pub(crate) fn invalidate_page(address: VirtualAddress) {
    #[cfg(not(test))]
    unsafe {
        asm!(
            "invlpg [{}]",
            in(reg) address.value(),
            options(nostack, preserves_flags),
        );
    }

    #[cfg(test)]
    let _ = address;
}
