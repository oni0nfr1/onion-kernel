use crate::boot::{AddressDecoder, kernel::KernelImageData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelImageError {
    MissingResponse,
    InvalidPhysicalAddress,
    InvalidVirtualAddress,
}

/// Collects the current kernel's load addresses from Limine and its section
/// boundaries from linker-defined symbols.
#[expect(unused_variables, reason = "implementation scaffold")]
pub fn current_kernel_image<P, V>(
    physical_address_decoder: &P,
    virtual_address_decoder: &V,
) -> Result<KernelImageData<P::Address, V::Address>, KernelImageError>
where
    P: AddressDecoder,
    V: AddressDecoder,
{
    todo!()
}
