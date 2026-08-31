use crate::arch::common::paging::Address;
use crate::boot::protocol::framebuffer::{FramebufferData, RgbPixelFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioCachePolicy {
    Uncached,
    WriteCombining,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalFramebufferError {
    ZeroWidth,
    ZeroHeight,
    InvalidPitch,
    UnsupportedPixelFormat,
    AddressOverflow,
    SizeOverflow,
}

/// Framebuffer information expressed only in terms of its physical aperture
/// and copied scalar metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalFramebuffer<P>
where
    P: Address,
{
    physical_start: P,
    byte_len: u64,
    width: usize,
    height: usize,
    pitch: usize,
    pixel_format: RgbPixelFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferMmioRequest<P>
where
    P: Address,
{
    physical_start: P,
    byte_len: u64,
    writable: bool,
    cache_policy: MmioCachePolicy,
}

/// Proof supplied by the mapping layer that a physical framebuffer aperture
/// is accessible through the kernel MMIO arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramebufferMmioMapping<P, V>
where
    P: Address,
    V: Address,
{
    physical_start: P,
    virtual_start: V,
    mapped_byte_len: u64,
    writable: bool,
    cache_policy: MmioCachePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramebufferResolveError {
    WrongPhysicalRegion,
    MappingTooSmall,
    MappingNotWritable,
    UnsupportedCachePolicy,
    AddressOverflow,
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<P> PhysicalFramebuffer<P>
where
    P: Address,
{
    pub fn new(
        physical_start: P,
        width: u64,
        height: u64,
        pitch: u64,
        pixel_format: RgbPixelFormat,
    ) -> Result<Self, PhysicalFramebufferError> {
        todo!()
    }

    pub fn physical_start(&self) -> P {
        todo!()
    }

    pub fn byte_len(&self) -> u64 {
        todo!()
    }

    pub fn mmio_request(&self, cache_policy: MmioCachePolicy) -> FramebufferMmioRequest<P> {
        todo!()
    }

    pub fn resolve<V>(
        self,
        mapping: FramebufferMmioMapping<P, V>,
    ) -> Result<FramebufferData<V>, FramebufferResolveError>
    where
        V: Address,
    {
        todo!()
    }
}

impl<P> FramebufferMmioRequest<P>
where
    P: Address,
{
    pub fn physical_start(&self) -> P {
        todo!()
    }

    pub fn byte_len(&self) -> u64 {
        todo!()
    }

    pub fn writable(&self) -> bool {
        todo!()
    }

    pub fn cache_policy(&self) -> MmioCachePolicy {
        todo!()
    }
}

#[expect(unused_variables, reason = "implementation scaffold")]
impl<P, V> FramebufferMmioMapping<P, V>
where
    P: Address,
    V: Address,
{
    /// Records a completed framebuffer MMIO mapping.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the supplied virtual range is currently
    /// mapped to the supplied physical range with the stated writability and
    /// cache policy, lies in the kernel MMIO arena, and remains mapped while
    /// the resulting runtime framebuffer is in use.
    pub unsafe fn new(
        physical_start: P,
        virtual_start: V,
        mapped_byte_len: u64,
        writable: bool,
        cache_policy: MmioCachePolicy,
    ) -> Self {
        todo!()
    }
}
