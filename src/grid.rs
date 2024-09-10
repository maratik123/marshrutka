use crate::cell::Cell;
use anyhow::{anyhow, Result};
use ndarray::{Array, Array2};
use tl::ParserOptions;

pub const CELL_SIZE: f32 = 89.0;

#[derive(Default)]
pub struct MapGrid {
    /// Row major
    pub grid: Array2<Cell>,
}

impl MapGrid {
    fn parse(s: &str) -> Result<Self> {
        let dom = tl::parse(s, ParserOptions::new())?;
        let parser = dom.parser();
        let map_grid = dom
            .get_elements_by_class_name("map-grid")
            .next()
            .ok_or_else(|| anyhow!("No map-grid elements found"))?;
        let map_grid = map_grid
            .get(parser)
            .and_then(|n| n.as_tag())
            .ok_or_else(|| anyhow!("Can not parse map-grid element"))?;
        map_grid.attributes()
        let cell_nodes = map_grid
            .children()
            .ok_or_else(|| anyhow!("map-grid element is not tag"))?
            .all(parser);
    }
}
