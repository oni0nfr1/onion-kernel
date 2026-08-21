pub mod onft;

pub const DEFAULT_FONT_DATA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/default.onft"
));

pub trait BitmapFont {
    type Glyph<'a>: BitmapGlyph
    where
        Self: 'a;

    fn metrics(&self) -> FontMetrics;

    fn glyph(&self, character: char) -> Option<Self::Glyph<'_>>;
}

pub trait BitmapGlyph {
    fn width(&self) -> usize;

    fn height(&self) -> usize;

    fn pixel(&self, x: usize, y: usize) -> Option<bool>;
}
pub struct FontMetrics {
    width: usize,
    height: usize,
}

impl FontMetrics {
    pub const fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }
}
