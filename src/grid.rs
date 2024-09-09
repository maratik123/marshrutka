use crate::cell::Cell;

pub const MAP_SIZE: usize = 13;
pub const CELL_SIZE: f32 = 90.0;
pub struct MapGrid {
    /// Row major
    pub grid: [[Cell; MAP_SIZE]; MAP_SIZE],
}
