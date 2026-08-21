pub mod color;
pub mod geometry;
pub mod grid;

pub fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(size_of::<u16>())?;
    let bytes: [u8; 2] = data.get(offset..end)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

pub fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(size_of::<u32>())?;
    let bytes: [u8; 4] = data.get(offset..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}
