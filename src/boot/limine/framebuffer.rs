use crate::boot::{AddressDecoder, framebuffer::PhysicalFramebuffer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramebufferError {
    MissingFramebufferResponse,
    MissingHhdmResponse,
    IndexOutOfBounds,
    AddressOutsideHhdm,
    InvalidPhysicalAddress,
    InvalidMetadata,
}

pub fn framebuffer_count() -> Result<usize, FramebufferError> {
    todo!()
}

/// Copies one Limine framebuffer descriptor and expresses its aperture as a
/// validated physical address. No pointer into the Limine address space is
/// retained in the returned value.
#[expect(unused_variables, reason = "implementation scaffold")]
pub fn physical_framebuffer<D>(
    index: usize,
    physical_address_decoder: &D,
) -> Result<PhysicalFramebuffer<D::Address>, FramebufferError>
where
    D: AddressDecoder,
{
    todo!()
}
