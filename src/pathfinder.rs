use crate::binary_heap::BinaryHeap;
use crate::consts::{CARAVAN_MONEY, CARAVAN_TIME, CARAVAN_TO_CENTER_MONEY, CARAVAN_TO_HOME_MONEY};
use crate::cost::{CaravanCost, CostComparator, EdgeCost, TotalCost};
use crate::grid::{MapGrid, PoI};
use crate::homeland::Homeland;
use crate::index::{Border, BorderDirection, CellIndex, CellIndexBuilder, Pos};
use crate::skill::{Fleetfoot, RouteGuru, Skill};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::iter;
use strum::IntoEnumIterator;

fn inflight_edges(
    grid: &MapGrid,
    homeland: Homeland,
    vertex: CellIndex,
    use_soe: bool,
    hq_position: Option<CellIndex>,
    use_caravans: bool,
    route_guru: RouteGuru,
) -> SmallVec<[(CellIndex, EdgeCost); 22]> {
    let mut ret = SmallVec::new();
    let homeland_size = grid.homeland_size();
    match vertex {
        CellIndex::Center => {
            // 4
            ret.extend(Border::iter().map(|border| {
                (
                    CellIndexBuilder::Border { border, shift: 1 }.build(),
                    EdgeCost::CentralMove,
                )
            }));
        }
        CellIndex::Border { border, shift } => {
            // 1
            ret.push(if shift == 1 {
                (CellIndex::Center, EdgeCost::CentralMove)
            } else {
                (
                    CellIndexBuilder::Border {
                        border,
                        shift: shift - 1,
                    }
                    .build(),
                    EdgeCost::StandardMove,
                )
            });
            // 0..1
            if (shift as usize) < homeland_size {
                ret.push((
                    CellIndexBuilder::Border {
                        border,
                        shift: shift + 1,
                    }
                    .build(),
                    EdgeCost::StandardMove,
                ));
            }
            // 2
            ret.extend(border.neighbours().map(|neighbour| {
                (
                    CellIndexBuilder::Homeland {
                        homeland: neighbour,
                        pos: border.direction().adjacent_pos_u8(shift),
                    }
                    .build(),
                    EdgeCost::StandardMove,
                )
            }));
        }
        CellIndex::Homeland {
            homeland: vertex_homeland,
            pos: Pos { x, y },
        } => {
            // 1
            ret.push((
                if x == 1 {
                    CellIndexBuilder::Border {
                        border: vertex_homeland.border(BorderDirection::Vertical),
                        shift: y,
                    }
                    .build()
                } else {
                    CellIndexBuilder::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x: x - 1, y },
                    }
                    .build()
                },
                EdgeCost::StandardMove,
            ));
            // 1
            ret.push((
                if y == 1 {
                    CellIndexBuilder::Border {
                        border: vertex_homeland.border(BorderDirection::Horizontal),
                        shift: x,
                    }
                    .build()
                } else {
                    CellIndexBuilder::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x, y: y - 1 },
                    }
                    .build()
                },
                EdgeCost::StandardMove,
            ));
            // 0..1
            if (x as usize) < grid.homeland_size() {
                ret.push((
                    CellIndexBuilder::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x: x + 1, y },
                    }
                    .build(),
                    EdgeCost::StandardMove,
                ));
            }
            // 0..1
            if (y as usize) < grid.homeland_size() {
                ret.push((
                    CellIndexBuilder::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x, y: y + 1 },
                    }
                    .build(),
                    EdgeCost::StandardMove,
                ));
            }
        }
    }
    // 0,16
    if use_caravans {
        let campfires = &grid.poi[PoI::Campfire];
        if vertex == CellIndex::Center || campfires.contains(&vertex) {
            for caravan_dest in iter::once(CellIndex::Center)
                .chain(campfires.iter().copied())
                .filter(|&campfire| campfire != vertex)
            {
                ret.push((
                    caravan_dest,
                    EdgeCost::Caravan(caravan_cost(
                        grid,
                        homeland,
                        vertex,
                        caravan_dest,
                        route_guru,
                    )),
                ));
            }
        }
    }
    // 0..1
    if use_soe {
        if let Some(nearest_campfire) = grid[&vertex]
            .nearest_campfire
            .get()
            .and_then(|nearest_campfire| nearest_campfire[homeland])
        {
            ret.push((nearest_campfire, EdgeCost::ScrollOfEscape));
        }
    }
    // 0..1
    if let Some(hq_position) = hq_position {
        ret.push((hq_position, EdgeCost::ScrollOfEscapeHQ));
    }
    ret
}

pub struct FindPathSettings<'a> {
    pub scroll_of_escape_cost: u32,
    pub scroll_of_escape_hq_cost: u32,
    pub use_soe: bool,
    pub use_caravans: bool,
    pub hq_position: Option<CellIndex>,
    pub route_guru: RouteGuru,
    pub fleetfoot: Fleetfoot,
    pub sort_by: (CostComparator, CostComparator),
    pub homeland: Homeland,
    pub grid: &'a MapGrid,
}

pub fn find_path(
    (from, to): (CellIndex, CellIndex),
    FindPathSettings {
        grid,
        scroll_of_escape_cost,
        scroll_of_escape_hq_cost,
        use_soe,
        hq_position,
        use_caravans,
        route_guru,
        fleetfoot,
        sort_by: (c1, c2),
        homeland,
    }: FindPathSettings,
) -> Option<TotalCost> {
    if from == to {
        return Some(TotalCost::new(from));
    }
    let mut dist: HashMap<_, _> = HashMap::new();
    let comparator = c1.and_then(c2);
    let mut heap: BinaryHeap<_, _> = BinaryHeap::new_by(|a, b| comparator(b, a));
    dist.insert(from, TotalCost::new(from));
    heap.push(TotalCost::new(from));
    while let Some(cost) = heap.pop() {
        let lowest_cost_index = cost.commands.last().unwrap().to;
        if lowest_cost_index == to {
            return Some(cost);
        }
        if comparator(&cost, &dist[&lowest_cost_index]).is_gt() {
            continue;
        }
        for (edge_index, edge_cost) in inflight_edges(
            grid,
            homeland,
            lowest_cost_index,
            use_soe,
            hq_position,
            use_caravans,
            route_guru,
        ) {
            let next = &cost
                + (
                    edge_cost,
                    scroll_of_escape_cost,
                    scroll_of_escape_hq_cost,
                    fleetfoot,
                    lowest_cost_index,
                    edge_index,
                );
            if dist
                .get(&edge_index).is_none_or(|old_cost| comparator(&next, old_cost).is_lt())
            {
                heap.push(next.clone());
                dist.insert(edge_index, next);
            }
        }
    }
    None
}

fn caravan_cost(
    grid: &MapGrid,
    homeland: Homeland,
    from: CellIndex,
    to: CellIndex,
    route_guru: RouteGuru,
) -> CaravanCost {
    let distance = grid[&from].distance(&grid[&to]) as u32;

    let money = match to {
        CellIndex::Center => CARAVAN_TO_CENTER_MONEY,
        CellIndex::Homeland {
            homeland: to_homeland,
            ..
        } if to_homeland == homeland => CARAVAN_TO_HOME_MONEY,
        _ => CARAVAN_MONEY,
    } * distance;

    let mut time = route_guru.time(CARAVAN_TIME).unwrap_or(CARAVAN_TIME);
    time *= distance;

    CaravanCost { money, time }
}

#[cfg(test)]
mod tests {
    use time::ext::NumericalDuration;

    #[test]
    fn time_as_str() {
        let m = 63.minutes();
        let s = 10.seconds();
        let ms = m + s;
        assert_eq!(ms.to_string(), "1h3m10s");
    }
}
