use crate::cell::{cell_parts, Cell, CellElement};
use crate::consts::{ARROW_TIP_CIRCLE, ARROW_WIDTH, CELL_SIZE, GRID_SPACING};
use crate::emoji::EmojiMap;
use crate::homeland::Homeland;
use crate::index::{BorderDirection, CellIndex, Pos};
use anyhow::{anyhow, Result};
use eframe::emath::Rot2;
use egui::ahash::HashSet;
use egui::ecolor::ParseHexColorError;
use egui::{Color32, Grid, InnerResponse, Painter, Pos2, ScrollArea, Stroke, Ui, Vec2};
use enum_map::{Enum, EnumMap};
use num_integer::Roots;
use simplecss::DeclarationTokenizer;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::iter;
use strum::{EnumCount, IntoEnumIterator};
use tl::HTMLTag;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug, EnumCount, Enum)]
pub enum PoI {
    Campfire,
    Fountain,
}

#[derive(Default)]
pub struct MapGrid {
    pub square_size: usize,
    /// Row major
    pub grid: Vec<Cell>,
    pub index: HashMap<CellIndex, usize>,
    pub poi: EnumMap<PoI, HashSet<CellIndex>>,
}

pub struct MapGridResponse {
    pub centers: HashMap<CellIndex, Pos2>,
    pub left: Option<CellIndex>,
    pub right: Option<CellIndex>,
}

impl MapGrid {
    pub fn parse(s: &str) -> Result<Self> {
        let dom = tl::parse(s, tl::ParserOptions::new())?;
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
        let max_coord = square_size / 2;
        let max_coord_i = max_coord as isize;
        let (mut grid, index): (Vec<_>, HashMap<_, _>) = map_cells
            .into_iter()
            .enumerate()
            .scan((-max_coord_i, -max_coord_i), |(x, y), item| {
                if x == &(max_coord_i + 1) {
                    *x = -max_coord_i;
                    *y += 1;
                }
                let ret = Some((*x, *y, item));
                *x += 1;
                ret
            })
            .map(|(x, y, (i, map_cell))| {
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
                let poi = cell_parts(&center);
                Ok((
                    Cell {
                        bg_color,
                        top_left,
                        top_right,
                        bottom_left,
                        bottom_right,
                        center,
                        index,
                        poi,
                        x: x as i8,
                        y: y as i8,
                        nearest_campfire: OnceCell::default(),
                    },
                    (index, i),
                ))
            })
            .collect::<Result<_>>()?;
        if let Some(i) = index.get(&CellIndex::Center) {
            let cell = &grid[*i];
            if cell.x != 0 || cell.y != 0 {
                return Err(anyhow!(
                    "Unexpected center position: ({}, {})",
                    cell.x,
                    cell.y
                ));
            }
        } else {
            return Err(anyhow!("Center is not found"));
        }
        let (poi, poi_by_homeland) = grid
            .iter()
            .filter_map(|cell| cell.poi.map(|poi| (poi, cell.index)))
            .fold(
                (EnumMap::default(), EnumMap::default()),
                |(mut acc_poi, mut acc_poi_by_homeland), (poi, cell_index)| {
                    let set: &mut HashSet<_> = &mut acc_poi[poi];
                    set.insert(cell_index);

                    if let CellIndex::Homeland { homeland, pos } = cell_index {
                        let map: &mut EnumMap<_, _> = &mut acc_poi_by_homeland[poi];
                        let set: &mut HashSet<_> = &mut map[homeland];
                        set.insert(pos);
                    }

                    (acc_poi, acc_poi_by_homeland)
                },
            );
        let nearest_campfires = Homeland::iter()
            .flat_map(|homeland| {
                let campfires = &poi_by_homeland[PoI::Campfire][homeland];
                let grid_ref = grid.as_slice();
                let index_ref = &index;
                let farland = homeland.farland();
                let (vert_border, vert_neighbour) =
                    homeland.neighbour_border(BorderDirection::Vertical);
                let (hor_border, hor_neighbour) =
                    homeland.neighbour_border(BorderDirection::Horizontal);
                let cached_nearest_campfires: HashMap<_, _> = [vert_border, hor_border]
                    .into_iter()
                    .flat_map(|border| {
                        (1..=(max_coord as u8))
                            .map(move |shift| CellIndex::Border { border, shift })
                    })
                    .chain(iter::once(CellIndex::Center))
                    .filter_map(|cell_index| {
                        nearest_campfire(cell_index, homeland, campfires, grid_ref, index_ref)
                            .map(|nearest_campfire| (cell_index, nearest_campfire))
                    })
                    .collect();

                grid_ref.iter().enumerate().filter_map(move |(i, cell)| {
                    cached_nearest_campfires
                        .get(&cell.index)
                        .copied()
                        .or_else(|| {
                            let Cell {
                                x: cell_x,
                                y: cell_y,
                                ..
                            } = cell;
                            let (cell_x, cell_y) = (*cell_x as isize, *cell_y as isize);
                            let (proj_x, proj_y) = match cell.index {
                                CellIndex::Homeland {
                                    homeland: cell_homeland,
                                    ..
                                } if cell_homeland == homeland => (cell_x, cell_y),
                                CellIndex::Homeland {
                                    homeland: cell_homeland,
                                    ..
                                } if cell_homeland == vert_neighbour => (0, cell_y),
                                CellIndex::Homeland {
                                    homeland: cell_homeland,
                                    ..
                                } if cell_homeland == hor_neighbour => (cell_x, 0),
                                CellIndex::Homeland {
                                    homeland: cell_homeland,
                                    ..
                                } if cell_homeland == farland => (0, 0),
                                CellIndex::Border { border, .. }
                                    if border != vert_border && border != hor_border =>
                                {
                                    (0, 0)
                                }
                                _ => unreachable!(),
                            };
                            let i = xy_to_i(max_coord_i, square_size, proj_x, proj_y);
                            nearest_campfire(
                                grid_ref[i].index,
                                homeland,
                                campfires,
                                grid_ref,
                                index_ref,
                            )
                        })
                        .map(|nearest_campfire| (i, homeland, nearest_campfire))
                })
            })
            .fold(
                vec![EnumMap::<Homeland, Option<CellIndex>>::default(); grid.len()],
                |mut acc, (i, homeland, nearest_campfire)| {
                    acc[i][homeland] = Some(nearest_campfire);
                    acc
                },
            );
        grid.iter_mut()
            .zip(nearest_campfires)
            .for_each(|(cell, nearest_campfire)| {
                cell.nearest_campfire.set(nearest_campfire).unwrap();
            });
        Ok(Self {
            square_size,
            grid,
            index,
            poi,
        })
    }

    pub fn ui_content(&self, ui: &mut Ui, emoji_map: &EmojiMap) -> InnerResponse<MapGridResponse> {
        Grid::new("map_grid")
            .striped(false)
            .spacing(Vec2::splat(GRID_SPACING))
            .min_col_width(CELL_SIZE)
            .min_row_height(CELL_SIZE)
            .show(ui, |ui| {
                let mut left = None;
                let mut right = None;
                let centers = self
                    .grid
                    .iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let center = ScrollArea::both()
                            .id_salt(i)
                            .show(ui, |ui| {
                                let (center, new_left, new_right) = cell.ui_content(ui, emoji_map);
                                if new_left.is_some() {
                                    left = new_left;
                                }
                                if new_right.is_some() {
                                    right = new_right;
                                }
                                center
                            })
                            .inner;
                        if (i + 1) % self.square_size == 0 {
                            ui.end_row();
                        }
                        (cell.index, center)
                    })
                    .collect();
                MapGridResponse {
                    centers,
                    left,
                    right,
                }
            })
    }

    pub const fn homeland_size(&self) -> usize {
        self.square_size / 2
    }
}

const fn xy_to_i(homeland_size: isize, square_size: usize, x: isize, y: isize) -> usize {
    (x + homeland_size) as usize + (y + homeland_size) as usize * square_size
}

fn nearest_campfire(
    from: CellIndex,
    homeland: Homeland,
    positions: &HashSet<Pos>,
    grid: &[Cell],
    index: &HashMap<CellIndex, usize>,
) -> Option<CellIndex> {
    if let CellIndex::Homeland {
        homeland: from_homeland,
        pos: from_pos,
    } = &from
    {
        if &homeland == from_homeland && positions.contains(from_pos) {
            return Some(from);
        }
    }
    let from_cell = &grid[index[&from]];
    positions
        .iter()
        .map(|&pos| CellIndex::Homeland { homeland, pos })
        .map(|campfire_index| index[&campfire_index])
        .map(|i| &grid[i])
        .min_by_key(|&campfire_cell| {
            (
                from_cell.distance(campfire_cell),
                (campfire_cell.x as isize).unsigned_abs(),
                (campfire_cell.y as isize).unsigned_abs(),
            )
        })
        .map(|campfire_cell| campfire_cell.index)
}

pub fn arrow(painter: &Painter, rot: Rot2, tip_length: f32, from: Pos2, to: Pos2, color: Color32) {
    let dir = tip_length * (to - from).normalized();
    let stroke = Stroke::new(ARROW_WIDTH, color);
    painter.line_segment([from, to], stroke);
    painter.line_segment([to, to - rot * dir], stroke);
    painter.line_segment([to, to - rot.inverse() * dir], stroke);
    painter.circle_stroke(from, ARROW_TIP_CIRCLE, stroke);
    painter.circle_filled(to, ARROW_TIP_CIRCLE, color);
}

fn parse_cell_element(map_cell: &HTMLTag, parser: &tl::Parser, class: &str) -> Option<CellElement> {
    map_cell.children().top().iter().find_map(|node_handle| {
        to_tag_with_class(node_handle, parser, class)
            .and_then(|html_tag| parse_text(html_tag, parser))
    })
}

fn to_tag_with_class<'p, 'buf>(
    node_handle: &tl::NodeHandle,
    parser: &'p tl::Parser<'buf>,
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

fn parse_text(html_tag: &HTMLTag, parser: &tl::Parser) -> Option<CellElement> {
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
