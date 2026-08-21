#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPosition {
    pub column: usize,
    pub row: usize,
}

impl GridPosition {
    pub const fn new(column: usize, row: usize) -> Self {
        Self { column, row }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub columns: usize,
    pub rows: usize,
}

impl GridSize {
    pub const fn new(columns: usize, rows: usize) -> Self {
        Self { columns, rows }
    }

    pub const fn contains(&self, position: GridPosition) -> bool {
        position.column < self.columns && position.row < self.rows
    }
}
