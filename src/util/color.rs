#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(u32);

impl Color {
    pub const BLACK: Self = Self::rgb(0x00, 0x00, 0x00);
    pub const WHITE: Self = Self::rgb(0xff, 0xff, 0xff);
    pub const RED: Self = Self::rgb(0xff, 0x00, 0x00);
    pub const GREEN: Self = Self::rgb(0x00, 0xff, 0x00);
    pub const BLUE: Self = Self::rgb(0x00, 0x00, 0xff);

    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self((red as u32) << 16 | (green as u32) << 8 | blue as u32)
    }

    pub const fn red(self) -> u8 {
        ((self.0 >> 16) & 0xff) as u8
    }

    pub const fn green(self) -> u8 {
        ((self.0 >> 8) & 0xff) as u8
    }

    pub const fn blue(self) -> u8 {
        (self.0 & 0xff) as u8
    }

    pub const fn from_rgb24(value: u32) -> Self {
        Self(value & 0x00_ff_ff_ff)
    }

    pub const fn as_rgb24(self) -> u32 {
        self.0
    }
}
