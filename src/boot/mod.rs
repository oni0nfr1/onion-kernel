//! Boot-protocol interpretation and early-resource preparation.
//!
//! Concrete interpreters translate bootloader-owned representations into the
//! bootloader-independent values in [`protocol`]. This module does not
//! construct drivers, allocate virtual regions, or modify page tables.

pub mod framebuffer;
pub mod kernel;
pub mod limine;
pub mod memory;
pub mod protocol;

use crate::arch::common::paging::Address;

/// Validates a raw address supplied by a boot protocol and constructs the
/// architecture-specific address type used by the kernel.
pub trait AddressDecoder {
    type Address: Address;

    fn decode(&self, value: u64) -> Option<Self::Address>;
}
