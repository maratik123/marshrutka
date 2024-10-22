use crate::binary_heap::BinaryHeap;
use crate::cost::{CaravanCost, CostComparator, EdgeCost, TotalCost};
use crate::grid::{MapGrid, PoI};
use crate::homeland::Homeland;
use crate::index::{Border, BorderDirection, CellIndex, Pos};
use crate::skill::RouteGuru;
use smallvec::SmallVec;
use std::collections::HashMap;
use strum::IntoEnumIterator;
use time::Duration;

fn inflight_edges(
    grid: &MapGrid,
    homeland: Homeland,
    vertex: CellIndex,
    use_soe: bool,
    use_caravans: bool,
    route_guru: RouteGuru,
) -> SmallVec<[(CellIndex, EdgeCost); 21]> {
    let mut ret = SmallVec::new();
    let homeland_size = grid.homeland_size();
    match vertex {
        CellIndex::Center => {
            // 4
            ret.extend(Border::iter().map(|border| {
                (
                    CellIndex::Border { border, shift: 1 },
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
                    CellIndex::Border {
                        border,
                        shift: shift - 1,
                    },
                    EdgeCost::StandardMove,
                )
            });
            // 0..1
            if (shift as usize) < homeland_size {
                ret.push((
                    CellIndex::Border {
                        border,
                        shift: shift + 1,
                    },
                    EdgeCost::StandardMove,
                ));
            }
            // 2
            ret.extend(border.neighbours().map(|neighbour| {
                (
                    CellIndex::Homeland {
                        homeland: neighbour,
                        pos: border.direction().adjacent_pos_u8(shift),
                    },
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
                    CellIndex::Border {
                        border: vertex_homeland.neighbour(BorderDirection::Vertical),
                        shift: y,
                    }
                } else {
                    CellIndex::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x: x - 1, y },
                    }
                },
                EdgeCost::StandardMove,
            ));
            // 1
            ret.push((
                if y == 1 {
                    CellIndex::Border {
                        border: vertex_homeland.neighbour(BorderDirection::Horizontal),
                        shift: x,
                    }
                } else {
                    CellIndex::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x, y: y - 1 },
                    }
                },
                EdgeCost::StandardMove,
            ));
            // 0..1
            if (x as usize) < grid.homeland_size() {
                ret.push((
                    CellIndex::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x: x + 1, y },
                    },
                    EdgeCost::StandardMove,
                ));
            }
            // 0..1
            if (y as usize) < grid.homeland_size() {
                ret.push((
                    CellIndex::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x, y: y + 1 },
                    },
                    EdgeCost::StandardMove,
                ));
            }
        }
    }
    if use_caravans {
        for poi in &grid.poi[PoI::Campfire] {}
    }
    // // 0..1
    // if use_soe {
    //     let homeland_campfire = grid.grid[grid.poi[&PoI::Campfire(homeland)]].index;
    //     if homeland_campfire != vertex {
    //         ret.push((homeland_campfire, &EdgeCost::ScrollOfEscape));
    //     }
    // }
    ret
}

pub fn find_path(
    grid: &MapGrid,
    homeland: Homeland,
    scroll_of_escape_cost: u32,
    (from, to): (CellIndex, CellIndex),
    (c1, c2): (CostComparator, CostComparator),
    use_soe: bool,
    use_caravans: bool,
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
        for (edge_index, edge_cost) in
            inflight_edges(grid, homeland, lowest_cost_index, use_soe, use_caravans)
        {
            let next = &cost
                + (
                    edge_cost,
                    scroll_of_escape_cost,
                    lowest_cost_index,
                    edge_index,
                );
            if dist
                .get(&edge_index)
                .map_or(true, |old_cost| comparator(&next, old_cost).is_lt())
            {
                heap.push(next.clone());
                dist.insert(edge_index, next);
            }
        }
    }
    None
}

const CARAVAN_TIME: Duration = Duration::minutes(4);
const CARAVAN_TO_HOME_MONEY: u32 = 2;
const CARAVAN_TO_CENTER_MONEY: u32 = 2;
const CARAVAN_MONEY: u32 = 5;

fn caravan_cost(
    grid: &MapGrid,
    homeland: Homeland,
    from: CellIndex,
    to: CellIndex,
    route_guru: RouteGuru,
) -> CaravanCost {
    let distance = grid.distance(grid.index[&from], grid.index[&to]) as u32;

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
