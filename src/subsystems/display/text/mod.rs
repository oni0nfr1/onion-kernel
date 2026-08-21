pub mod framebuffer;

pub use framebuffer::FramebufferTextScreen;

use crate::util::{
    color::Color,
    grid::{GridPosition, GridSize},
};

pub trait TextScreen {
    type Error;

    /// 셀 그리드의 열과 행 수를 반환합니다.
    fn size(&self) -> GridSize;

    /// 지정한 셀 전체를 `background`로 채운 후 글자를 그립니다.
    fn put_char(
        &mut self,
        position: GridPosition,
        ch: char,
        foreground: Color,
        background: Color,
    ) -> Result<(), Self::Error>;

    /// 지정한 셀 전체를 단색으로 채웁니다.
    fn fill_cell(&mut self, position: GridPosition, color: Color) -> Result<(), Self::Error>;

    /// 그리드와 그리드에 포함되지 않는 나머지 영역을 모두 채웁니다.
    fn clear(&mut self, color: Color);

    /// 기존 내용을 행 단위로 이동합니다.
    ///
    /// - `rows > 0`: 내용을 더 작은 행 인덱스 방향, 즉 위로 이동
    /// - `rows < 0`: 내용을 더 큰 행 인덱스 방향, 즉 아래로 이동
    /// - 새로 노출된 영역은 `erase_color`로 채움
    fn scroll(&mut self, rows: isize, erase_color: Color);
}
