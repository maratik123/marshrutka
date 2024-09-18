use crate::cell::{Cell, CellElement};
use crate::consts::{CELL_SIZE, GRID_SPACING};
use crate::emoji::{EmojiCode, EmojiMap};
use crate::homeland::Homeland;
use crate::index::CellIndex;
use anyhow::{anyhow, Result};
use egui::ecolor::ParseHexColorError;
use egui::{Color32, Grid, ScrollArea, Ui, Vec2};
use num_integer::Roots;
use simplecss::DeclarationTokenizer;
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use tl::{HTMLTag, NodeHandle, Parser, ParserOptions};

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum PoI {
    Forum,
    Campfire(Homeland),
}

#[derive(Default)]
pub struct MapGrid {
    pub square_size: usize,
    /// Row major
    pub grid: Vec<Cell>,
    pub index: HashMap<CellIndex, usize>,
    pub poi: HashMap<PoI, usize>,
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
        let (grid, index): (Vec<_>, HashMap<_, _>) = map_cells
            .into_iter()
            .enumerate()
            .map(|(i, map_cell)| {
                let bg_color = parse_bg_color_from_style(map_cell)?;
                let top_left = parse_cell_element(map_cell, parser, "top-left-text");
                let top_right = parse_cell_element(map_cell, parser, "top-right-text");
                let bottom_left = parse_cell_element(map_cell, parser, "bottom-left-text");
                let bottom_right = parse_cell_element(map_cell, parser, "bottom-right-text");
                let center = parse_text(map_cell, parser);
                let index = (
                    bottom_right.as_ref().map(Cow::from),
                    top_right.as_ref().map(Cow::from),
                )
                    .try_into()
                    .map_err(|_| {
                        anyhow!(
                            "Can not index cell {} {}",
                            bottom_right
                                .as_ref()
                                .map(|s| s.to_string())
                                .unwrap_or_default(),
                            top_right
                                .as_ref()
                                .map(|s| s.to_string())
                                .unwrap_or_default()
                        )
                    })?;
                Ok((
                    Cell {
                        bg_color,
                        top_left,
                        top_right,
                        bottom_left,
                        bottom_right,
                        center,
                        index,
                    },
                    (index, i),
                ))
            })
            .collect::<Result<_>>()?;
        let poi = grid
            .iter()
            .enumerate()
            .filter_map(|(i, cell)| {
                Some((
                    match cell {
                        Cell {
                            center: Some(CellElement::Emoji(EmojiCode('\u{1f3db}', None))),
                            ..
                        } => PoI::Forum,
                        Cell {
                            center: Some(CellElement::Emoji(EmojiCode('\u{1f525}', None))),
                            bottom_right: Some(CellElement::Emoji(EmojiCode(ch, None))),
                            ..
                        } => PoI::Campfire(Homeland::try_from(*ch).ok()?),
                        _ => None?,
                    },
                    i,
                ))
            })
            .collect();
        Ok(Self {
            square_size,
            grid,
            index,
            poi,
        })
    }

    pub fn ui_content(
        &self,
        ui: &mut Ui,
        emoji_map: &EmojiMap,
    ) -> (Option<CellIndex>, Option<CellIndex>) {
        Grid::new("map_grid")
            .striped(false)
            .spacing(Vec2::splat(GRID_SPACING))
            .min_col_width(CELL_SIZE)
            .min_row_height(CELL_SIZE)
            .show(ui, |ui| {
                let mut left = None;
                let mut right = None;
                for (i, cell) in self.grid.iter().enumerate() {
                    ScrollArea::both().id_source(i).show(ui, |ui| {
                        let (new_left, new_right) =
                            cell.ui_content(ui, emoji_map, || self.grid[i].index);
                        if new_left.is_some() {
                            left = new_left;
                        }
                        if new_right.is_some() {
                            right = new_right;
                        }
                    });
                    if (i + 1) % self.square_size == 0 {
                        ui.end_row();
                    }
                }
                (left, right)
            })
            .inner
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
        .is_some_and(|bytes| bytes == class)
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
