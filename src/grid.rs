use crate::cell::{Cell, CellElement};
use crate::emoji::EmojiMap;
use anyhow::{anyhow, Result};
use egui::ecolor::ParseHexColorError;
use egui::{Color32, Grid, ScrollArea, Ui, Vec2};
use num_integer::Roots;
use simplecss::DeclarationTokenizer;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use tl::{HTMLTag, NodeHandle, Parser, ParserOptions};

pub const CELL_SIZE: f32 = 88.0;

#[derive(Default)]
pub struct MapGrid {
    /// Row major
    pub grid: Vec<Cell>,
    pub square_size: usize,
}

impl MapGrid {
    pub fn parse(s: &str) -> Result<Self> {
        let dom = tl::parse(s, ParserOptions::new())?;
        let parser = dom.parser();
        let map_grid = dom
            .get_elements_by_class_name("map-grid")
            .find_map(|node_handle| node_handle.get(parser).and_then(|node| node.as_tag()))
            .ok_or_else(|| anyhow!("No map-grid elements found"))?;
        let map_cells: Vec<_> = map_grid
            .children()
            .top()
            .iter()
            .filter_map(|node_handle| to_tag_with_class(node_handle, parser, "map-cell"))
            .collect();
        let square_size = map_cells.len().sqrt();
        if square_size * square_size != map_cells.len() {
            return Err(anyhow!("Map grid is not square: {}", map_cells.len()));
        }
        Ok(Self {
            square_size,
            grid: map_cells
                .into_iter()
                .map(|map_cell| {
                    Ok(Cell {
                        bg_color: parse_bg_color_from_style(map_cell)?,
                        top_left: parse_cell_element(map_cell, parser, "top-left-text"),
                        top_right: parse_cell_element(map_cell, parser, "top-right-text"),
                        bottom_left: parse_cell_element(map_cell, parser, "bottom-left-text"),
                        bottom_right: parse_cell_element(map_cell, parser, "bottom-right-text"),
                        center: parse_text(map_cell, parser),
                    })
                })
                .collect::<Result<_>>()?,
        })
    }

    fn i_to_name(&self, i: usize) -> String {
        let cell = &self.grid[i];
        match (&cell.bottom_right, &cell.top_right) {
            (Some(bottom_right), Some(top_right)) => format!("{bottom_right} {top_right}"),
            (Some(bottom_right), None) => bottom_right.to_string(),
            (None, Some(top_right)) => top_right.to_string(),
            (None, None) => String::new(),
        }
    }

    pub fn ui_content(
        &self,
        ui: &mut Ui,
        emoji_map: &EmojiMap,
        #[allow(clippy::ptr_arg)] left: &mut String,
        #[allow(clippy::ptr_arg)] right: &mut String,
    ) {
        Grid::new("map_grid")
            .striped(false)
            .spacing(Vec2::splat(2.0))
            .min_col_width(CELL_SIZE)
            .min_row_height(CELL_SIZE)
            .show(ui, |ui| {
                for (i, cell) in self.grid.iter().enumerate() {
                    ScrollArea::both().id_source(i).show(ui, |ui| {
                        cell.ui_content(ui, emoji_map, left, right, || self.i_to_name(i));
                    });
                    if (i + 1) % self.square_size == 0 {
                        ui.end_row();
                    }
                }
            });
    }
}

fn parse_cell_element(map_cell: &HTMLTag, parser: &Parser, class: &str) -> Option<CellElement> {
    map_cell.children().top().iter().find_map(|node_handle| {
        to_tag_with_class(node_handle, parser, class)
            .and_then(|html_tag| parse_text(html_tag, parser))
    })
}

fn to_tag_with_class<'p, 'buf>(
    node_handle: &NodeHandle,
    parser: &'p Parser<'buf>,
    class: &str,
) -> Option<&'p HTMLTag<'buf>> {
    node_handle
        .get(parser)
        .and_then(|node| node.as_tag())
        .filter(|html_tag| has_class(html_tag, class))
}

fn has_class(html_tag: &HTMLTag, class: &str) -> bool {
    html_tag
        .attributes()
        .class()
        .filter(|&bytes| bytes == class)
        .is_some()
}

fn parse_text(html_tag: &HTMLTag, parser: &Parser) -> Option<CellElement> {
    html_tag.children().top().iter().find_map(|node_handle| {
        node_handle
            .get(parser)
            .and_then(|node| node.as_raw())
            .and_then(|bytes| bytes.as_utf8_str().trim().try_into().ok())
    })
}

fn parse_bg_color_from_style(html_tag: &HTMLTag) -> Result<Option<Color32>> {
    html_tag
        .attributes()
        .get("style")
        .flatten()
        .and_then(|style| {
            let style = style.as_utf8_str();
            DeclarationTokenizer::from(style.as_ref())
                .find(|v| v.name == "background-color")
                .map(|v| Color32::from_hex(v.value).map_err(ParseHexColorErrorWrapper))
        })
        .transpose()
        .map_err(|e| e.into())
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
