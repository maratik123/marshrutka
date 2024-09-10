use crate::cell::Cell;

pub const MAP_SIZE: usize = 13;
pub const CELL_SIZE: f32 = 89.0;

#[derive(Default)]
pub struct MapGrid {
    /// Row major
    pub grid: [[Cell; MAP_SIZE]; MAP_SIZE],
}

impl MapGrid {
    fn parse(s: &str) -> Option<Self> {
        None
    }
}
