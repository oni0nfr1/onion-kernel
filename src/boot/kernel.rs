use crate::arch::common::paging::Address;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelImageDataError {
    AddressOverflow,
}

/// Physical and boot-time virtual bases reported for the loaded kernel image.
/// Section boundaries and final permissions are supplied separately by the
/// linker and architecture paging bootstrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelImageData<P, V>
where
    P: Address,
    V: Address,
{
    physical_base: P,
    virtual_base: V,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<P, V> KernelImageData<P, V>
where
    P: Address,
    V: Address,
{
    pub fn new(physical_base: P, virtual_base: V) -> Result<Self, KernelImageDataError> {
        todo!()
    }

    pub fn physical_base(&self) -> P {
        todo!()
    }

    pub fn virtual_base(&self) -> V {
        todo!()
    }
}
