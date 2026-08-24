pub mod ascii;
pub mod buffer;

pub use ascii::{AsciiConsole, AsciiConsoleError};

use crate::util::{
    color::Color,
    grid::{GridPosition, GridSize},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsoleColors {
    pub foreground: Color,
    pub background: Color,
}

impl ConsoleColors {
    pub const fn new(foreground: Color, background: Color) -> Self {
        Self {
            foreground,
            background,
        }
    }
}

impl Default for ConsoleColors {
    fn default() -> Self {
        Self::new(Color::WHITE, Color::BLACK)
    }
}

pub trait Console {
    type Error;

    /// 바이트 하나를 콘솔의 제어 문자 및 문자 인코딩 규칙에 따라 처리합니다.
    fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error>;

    /// 바이트를 순서대로 처리합니다.
    ///
    /// 오류가 발생하면 그 전에 성공적으로 처리한 바이트의 효과는 유지됩니다.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        for &byte in bytes {
            self.write_byte(byte)?;
        }
        Ok(())
    }

    fn size(&self) -> GridSize;

    fn cursor_position(&self) -> GridPosition;

    /// 다음 출력 위치를 설정합니다.
    ///
    /// 화면 그리드 밖의 위치는 오류로 처리해야 합니다.
    fn set_cursor_position(&mut self, position: GridPosition) -> Result<(), Self::Error>;

    fn colors(&self) -> ConsoleColors;

    fn set_colors(&mut self, colors: ConsoleColors);

    /// 현재 배경색으로 화면을 지우고 커서를 왼쪽 위로 옮깁니다.
    fn clear(&mut self);
}
