use crate::{
    drivers::framebuffer::{Framebuffer, PixelDrawError},
    subsystems::display::font::{BitmapFont, BitmapGlyph},
    util::{
        color::Color,
        geometry::{Point, Rect, Scale},
        grid::{GridPosition, GridSize},
    },
};

use super::TextScreen;

type PixelPoint = Point<usize>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphDrawError {
    OutOfBounds,
    NoGlyph,
    InvalidGlyph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramebufferTextScreenCreateError {
    InvalidFontMetrics,
    NoUsableCells,
    SizeOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramebufferTextScreenError {
    OutOfBounds,
    NoGlyph,
    InvalidGlyph,
}

impl From<PixelDrawError> for FramebufferTextScreenError {
    fn from(_: PixelDrawError) -> Self {
        Self::OutOfBounds
    }
}

impl From<GlyphDrawError> for FramebufferTextScreenError {
    fn from(error: GlyphDrawError) -> Self {
        match error {
            GlyphDrawError::OutOfBounds => Self::OutOfBounds,
            GlyphDrawError::NoGlyph => Self::NoGlyph,
            GlyphDrawError::InvalidGlyph => Self::InvalidGlyph,
        }
    }
}

pub trait FramebufferTextExt {
    fn put_char<F>(
        &mut self,
        font: &F,
        ch: char,
        position: PixelPoint,
        color: Color,
    ) -> Result<(), GlyphDrawError>
    where
        F: BitmapFont;

    fn put_char_opaque<F>(
        &mut self,
        font: &F,
        ch: char,
        position: PixelPoint,
        foreground: Color,
        background: Color,
    ) -> Result<(), GlyphDrawError>
    where
        F: BitmapFont;
}

impl FramebufferTextExt for Framebuffer<'_> {
    fn put_char<F>(
        &mut self,
        font: &F,
        ch: char,
        position: PixelPoint,
        color: Color,
    ) -> Result<(), GlyphDrawError>
    where
        F: BitmapFont,
    {
        let glyph = font.glyph(ch).ok_or(GlyphDrawError::NoGlyph)?;

        let right = position
            .x
            .checked_add(glyph.width())
            .ok_or(GlyphDrawError::OutOfBounds)?;
        let bottom = position
            .y
            .checked_add(glyph.height())
            .ok_or(GlyphDrawError::OutOfBounds)?;

        if right > self.width || bottom > self.height {
            return Err(GlyphDrawError::OutOfBounds);
        }

        for glyph_y in 0..glyph.height() {
            for glyph_x in 0..glyph.width() {
                match glyph.pixel(glyph_x, glyph_y) {
                    Some(true) => {
                        // SAFETY: 위의 범위 검사로 글리프의 모든 픽셀이 프레임버퍼 안에 있다.
                        unsafe {
                            self.put_pixel_unchecked(
                                PixelPoint::new(position.x + glyph_x, position.y + glyph_y),
                                color,
                            );
                        }
                    }
                    Some(false) => {}
                    None => return Err(GlyphDrawError::InvalidGlyph),
                }
            }
        }

        Ok(())
    }

    fn put_char_opaque<F>(
        &mut self,
        font: &F,
        ch: char,
        position: PixelPoint,
        foreground: Color,
        background: Color,
    ) -> Result<(), GlyphDrawError>
    where
        F: BitmapFont,
    {
        let glyph = font.glyph(ch).ok_or(GlyphDrawError::NoGlyph)?;

        let right = position
            .x
            .checked_add(glyph.width())
            .ok_or(GlyphDrawError::OutOfBounds)?;
        let bottom = position
            .y
            .checked_add(glyph.height())
            .ok_or(GlyphDrawError::OutOfBounds)?;

        if right > self.width || bottom > self.height {
            return Err(GlyphDrawError::OutOfBounds);
        }

        for glyph_y in 0..glyph.height() {
            for glyph_x in 0..glyph.width() {
                let color = match glyph.pixel(glyph_x, glyph_y) {
                    Some(true) => foreground,
                    Some(false) => background,
                    None => return Err(GlyphDrawError::InvalidGlyph),
                };

                // SAFETY: 위의 범위 검사로 글리프의 모든 픽셀이 프레임버퍼 안에 있다.
                unsafe {
                    self.put_pixel_unchecked(
                        PixelPoint::new(position.x + glyph_x, position.y + glyph_y),
                        color,
                    );
                }
            }
        }

        Ok(())
    }
}

pub struct FramebufferTextScreen<'framebuffer, F>
where
    F: BitmapFont,
{
    framebuffer: Framebuffer<'framebuffer>,
    font: F,
    cell_size: Scale<usize>,
    size: GridSize,
    grid_width: usize,
    grid_height: usize,
}

impl<'framebuffer, F> FramebufferTextScreen<'framebuffer, F>
where
    F: BitmapFont,
{
    pub fn new(
        framebuffer: Framebuffer<'framebuffer>,
        font: F,
    ) -> Result<Self, FramebufferTextScreenCreateError> {
        let metrics = font.metrics();
        if metrics.width() == 0 || metrics.height() == 0 {
            return Err(FramebufferTextScreenCreateError::InvalidFontMetrics);
        }

        let cell_size = Scale::new(metrics.width(), metrics.height());

        let columns = framebuffer.width / cell_size.width;
        let rows = framebuffer.height / cell_size.height;
        if columns == 0 || rows == 0 {
            return Err(FramebufferTextScreenCreateError::NoUsableCells);
        }

        let grid_width = columns
            .checked_mul(cell_size.width)
            .ok_or(FramebufferTextScreenCreateError::SizeOverflow)?;
        let grid_height = rows
            .checked_mul(cell_size.height)
            .ok_or(FramebufferTextScreenCreateError::SizeOverflow)?;

        Ok(Self {
            framebuffer,
            font,
            cell_size,
            size: GridSize::new(columns, rows),
            grid_width,
            grid_height,
        })
    }

    pub const fn cell_size(&self) -> Scale<usize> {
        self.cell_size
    }

    pub fn into_parts(self) -> (Framebuffer<'framebuffer>, F) {
        (self.framebuffer, self.font)
    }

    fn cell_region(&self, position: GridPosition) -> Option<Rect<usize>> {
        if !self.size.contains(position) {
            return None;
        }

        let x = position.column * self.cell_size.width;
        let y = position.row * self.cell_size.height;

        // SAFETY: `position` is inside the text grid. `new` checked that the
        // complete grid dimensions (`columns * cell width` and `rows * cell
        // height`) fit in `usize`, so this cell's origin and far edges do too.
        Some(unsafe { Rect::new_unchecked(x, y, self.cell_size.width, self.cell_size.height) })
    }

    fn fill_grid(&mut self, color: Color) {
        let Some(region) = Rect::new(0, 0, self.grid_width, self.grid_height) else {
            return;
        };
        let result = self.framebuffer.fill_rect(region, color);
        debug_assert!(
            result.is_ok(),
            "text grid region {region:?} must fit inside the owned framebuffer, got {result:?}",
        );
    }
}

impl<F> TextScreen for FramebufferTextScreen<'_, F>
where
    F: BitmapFont,
{
    type Error = FramebufferTextScreenError;

    fn size(&self) -> GridSize {
        self.size
    }

    fn put_char(
        &mut self,
        position: GridPosition,
        ch: char,
        foreground: Color,
        background: Color,
    ) -> Result<(), Self::Error> {
        let cell_region = self
            .cell_region(position)
            .ok_or(FramebufferTextScreenError::OutOfBounds)?;
        self.framebuffer
            .put_char_opaque(
                &self.font,
                ch,
                PixelPoint::new(cell_region.x(), cell_region.y()),
                foreground,
                background,
            )
            .map_err(Into::into)
    }

    fn fill_cell(&mut self, position: GridPosition, color: Color) -> Result<(), Self::Error> {
        let cell_region = self
            .cell_region(position)
            .ok_or(FramebufferTextScreenError::OutOfBounds)?;

        self.framebuffer
            .fill_rect(cell_region, color)
            .map_err(Into::into)
    }

    fn clear(&mut self, color: Color) {
        self.framebuffer.clear(color);
    }

    fn scroll(&mut self, rows: isize, erase_color: Color) {
        let row_count = rows.unsigned_abs().min(self.size.rows);
        if row_count == 0 {
            return;
        }
        if row_count == self.size.rows {
            self.fill_grid(erase_color);
            return;
        }

        let Some(pixel_rows) = row_count.checked_mul(self.cell_size.height) else {
            return;
        };
        let Some(retained_height) = self.grid_height.checked_sub(pixel_rows) else {
            return;
        };

        let regions = if rows > 0 {
            let Some(source) = Rect::new(0, pixel_rows, self.grid_width, retained_height) else {
                return;
            };
            let Some(exposed) = Rect::new(0, retained_height, self.grid_width, pixel_rows) else {
                return;
            };
            (source, PixelPoint::new(0, 0), exposed)
        } else {
            let Some(source) = Rect::new(0, 0, self.grid_width, retained_height) else {
                return;
            };
            let Some(exposed) = Rect::new(0, 0, self.grid_width, pixel_rows) else {
                return;
            };
            (source, PixelPoint::new(0, pixel_rows), exposed)
        };
        let (source, destination, exposed) = regions;

        let copy_result = self.framebuffer.copy_rect(source, destination);
        debug_assert!(
            copy_result.is_ok(),
            "scroll copy must remain inside the text grid: source={source:?}, destination={destination:?}, result={copy_result:?}",
        );

        let fill_result = self.framebuffer.fill_rect(exposed, erase_color);
        debug_assert!(
            fill_result.is_ok(),
            "scroll-exposed region {exposed:?} must fit inside the owned framebuffer, got {fill_result:?}",
        );
    }
}
