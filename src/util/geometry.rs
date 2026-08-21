#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scale<T> {
    pub width: T,
    pub height: T,
}

impl<T> Scale<T> {
    pub const fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned region whose right and bottom edges fit in `usize`.
///
/// Fields are private so every value preserves the checked-construction
/// invariant used by `right` and `bottom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect<T> {
    x: T,
    y: T,
    width: T,
    height: T,
}

impl Rect<usize> {
    pub const fn new(x: usize, y: usize, width: usize, height: usize) -> Option<Self> {
        if x.checked_add(width).is_none() || y.checked_add(height).is_none() {
            return None;
        }

        Some(Self {
            x,
            y,
            width,
            height,
        })
    }

    /// Creates a rectangle without checking its right and bottom edges.
    ///
    /// # Safety
    ///
    /// `x + width` and `y + height` must both fit in `usize`. This preserves
    /// the invariant relied on by `right` and `bottom`.
    pub(crate) const unsafe fn new_unchecked(
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn from_origin_size(origin: Point<usize>, size: Scale<usize>) -> Option<Self> {
        Self::new(origin.x, origin.y, size.width, size.height)
    }

    pub fn from_corners(p1: Point<usize>, p2: Point<usize>) -> Option<Self> {
        let x = p1.x.min(p2.x);
        let y = p1.y.min(p2.y);
        let right = p1.x.max(p2.x);
        let bottom = p1.y.max(p2.y);

        Self::new(x, y, right - x, bottom - y)
    }

    pub const fn x(self) -> usize {
        self.x
    }

    pub const fn y(self) -> usize {
        self.y
    }

    pub const fn width(self) -> usize {
        self.width
    }

    pub const fn height(self) -> usize {
        self.height
    }

    pub const fn left(self) -> usize {
        self.x
    }

    pub const fn top(self) -> usize {
        self.y
    }

    pub const fn right(self) -> usize {
        // `new` verifies this sum and private fields preserve that invariant.
        self.x + self.width
    }

    pub const fn bottom(self) -> usize {
        // `new` verifies this sum and private fields preserve that invariant.
        self.y + self.height
    }
}
