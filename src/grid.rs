use crate::cell::Cell;
use anyhow::{anyhow, Result};
use egui::ecolor::ParseHexColorError;
use egui::Color32;
use ndarray::{Array, Array2};
use num_integer::Roots;
use simplecss::DeclarationTokenizer;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use tl::ParserOptions;

pub const CELL_SIZE: f32 = 88.0;

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
            .and_then(|n| n.as_tag().filter(|tag| tag.name() == "div"))
            .ok_or_else(|| anyhow!("Can not parse map-grid element"))?;
        let color = map_grid
            .attributes()
            .get("style")
            .flatten()
            .map(|style| {
                let style = style.as_utf8_str();
                DeclarationTokenizer::from(style.as_ref())
                    .find(|v| v.name == "background-color")
                    .map(|v| Color32::from_hex(v.value).map_err(|e| ParseHexColorErrorWrapper(e)))
            })
            .flatten()
            .transpose()?;
        let map_cells: Vec<_> = map_grid
            .children()
            .top()
            .iter()
            .flat_map(|c| c.get(parser).and_then(|n| n.as_tag()))
            .collect();
        let square_size = map_cells.len().sqrt();
        if square_size * square_size != map_cells.len() {
            return Err(anyhow!(""));
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParseHexColorErrorWrapper(ParseHexColorError);

impl Error for ParseHexColorErrorWrapper {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let ParseHexColorError::InvalidInt(e) = &self.0 {
            Some(e)
        } else {
            None
        }
    }
}

impl Display for ParseHexColorErrorWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
