mod private {
    pub trait Sealed {}
}

pub trait TableLevel: private::Sealed + Clone + Copy + core::fmt::Debug + PartialEq + Eq {}

pub trait NonLeafTableLevel: TableLevel {
    type Next: TableLevel;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level1;

impl private::Sealed for Level4 {}
impl private::Sealed for Level3 {}
impl private::Sealed for Level2 {}
impl private::Sealed for Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

impl NonLeafTableLevel for Level4 {
    type Next = Level3;
}

impl NonLeafTableLevel for Level3 {
    type Next = Level2;
}

impl NonLeafTableLevel for Level2 {
    type Next = Level1;
}
