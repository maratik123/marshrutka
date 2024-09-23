use crate::binary_heap::BinaryHeap;
use crate::cost::{
    CostComparator, EdgeCost, TotalCost, CARAVAN_CAMPFIRE_CENTER, CARAVAN_CENTER_ENEMY,
    CARAVAN_CENTER_HOMELAND, CARAVAN_FARLAND_HOMELAND, CARAVAN_FARLAND_NEIGHBOUR,
    CARAVAN_HOMELAND_FARLAND, CARAVAN_HOMELAND_NEIGHBOUR, CARAVAN_NEIGHBOUR_FARLAND,
    CARAVAN_NEIGHBOUR_HOMELAND, CARAVAN_NEIGHBOUR_NEIGHBOUR,
};
use crate::grid::{MapGrid, PoI};
use crate::homeland::Homeland;
use crate::index::{Border, BorderDirection, CellIndex, Pos};
use arrayvec::ArrayVec;
use std::collections::HashMap;
use std::iter;
use strum::IntoEnumIterator;

type EdgeCostRef = &'static EdgeCost;

fn inflight_edges(
    grid: &MapGrid,
    homeland: Homeland,
    vertex: CellIndex,
) -> ArrayVec<(CellIndex, EdgeCostRef), 9> {
    let mut ret = ArrayVec::new();
    let homeland_size = grid.homeland_size();
    match vertex {
        CellIndex::Center => {
            // 4
            ret.extend(Border::iter().map(|border| {
                (
                    CellIndex::Border { border, shift: 1 },
                    &EdgeCost::CentralMove,
                )
            }));
            // 3
            ret.extend(
                homeland
                    .neighbours()
                    .into_iter()
                    .chain(iter::once(homeland.farland()))
                    .map(|enemy| {
                        (
                            grid.grid[grid.poi[&PoI::Campfire(enemy)]].index,
                            CARAVAN_CENTER_ENEMY,
                        )
                    }),
            );
            // 1
            ret.push((
                grid.grid[grid.poi[&PoI::Campfire(homeland)]].index,
                CARAVAN_CENTER_HOMELAND,
            ));
        }
        CellIndex::Border { border, shift } => {
            // 1
            ret.push(if shift == 1 {
                (CellIndex::Center, &EdgeCost::CentralMove)
            } else {
                (
                    CellIndex::Border {
                        border,
                        shift: shift - 1,
                    },
                    &EdgeCost::StandardMove,
                )
            });
            // 0..1
            if (shift as usize) < homeland_size {
                ret.push((
                    CellIndex::Border {
                        border,
                        shift: shift + 1,
                    },
                    &EdgeCost::StandardMove,
                ));
            }
            // 2
            ret.extend(border.neighbours().map(|neighbour| {
                (
                    CellIndex::Homeland {
                        homeland: neighbour,
                        pos: border.direction().adjacent_pos_u8(shift),
                    },
                    &EdgeCost::StandardMove,
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
                &EdgeCost::StandardMove,
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
                &EdgeCost::StandardMove,
            ));
            // 0..1
            if (x as usize) < grid.homeland_size() {
                ret.push((
                    CellIndex::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x: x + 1, y },
                    },
                    &EdgeCost::StandardMove,
                ));
            }
            // 0..1
            if (y as usize) < grid.homeland_size() {
                ret.push((
                    CellIndex::Homeland {
                        homeland: vertex_homeland,
                        pos: Pos { x, y: y + 1 },
                    },
                    &EdgeCost::StandardMove,
                ));
            }
            if let Some(PoI::Campfire(campfire)) = grid.grid[grid.index[&vertex]].poi {
                // 1
                ret.push((CellIndex::Center, CARAVAN_CAMPFIRE_CENTER));
                fn add_neighbour_paths(
                    arr: &mut ArrayVec<(CellIndex, EdgeCostRef), 9>,
                    grid: &MapGrid,
                    campfire: Homeland,
                    edge_cost: EdgeCostRef,
                ) {
                    arr.extend(campfire.neighbours().into_iter().map(|neighbour_campfire| {
                        (
                            grid.grid[grid.poi[&PoI::Campfire(neighbour_campfire)]].index,
                            edge_cost,
                        )
                    }));
                }
                if campfire == homeland {
                    // 1
                    ret.push((
                        grid.grid[grid.poi[&PoI::Campfire(campfire.farland())]].index,
                        CARAVAN_HOMELAND_FARLAND,
                    ));
                    // 2
                    add_neighbour_paths(&mut ret, grid, campfire, CARAVAN_HOMELAND_NEIGHBOUR);
                } else if campfire == homeland.farland() {
                    // 1
                    ret.push((
                        grid.grid[grid.poi[&PoI::Campfire(homeland)]].index,
                        CARAVAN_FARLAND_HOMELAND,
                    ));
                    // 2
                    add_neighbour_paths(&mut ret, grid, campfire, CARAVAN_FARLAND_NEIGHBOUR);
                } else {
                    // 1
                    ret.push((
                        grid.grid[grid.poi[&PoI::Campfire(homeland)]].index,
                        CARAVAN_NEIGHBOUR_HOMELAND,
                    ));
                    // 1
                    ret.push((
                        grid.grid[grid.poi[&PoI::Campfire(campfire.farland())]].index,
                        CARAVAN_NEIGHBOUR_NEIGHBOUR,
                    ));
                    // 1
                    ret.push((
                        grid.grid[grid.poi[&PoI::Campfire(homeland.farland())]].index,
                        CARAVAN_NEIGHBOUR_FARLAND,
                    ));
                }
            }
        }
    }
    // 0..1
    let homeland_campfire = grid.grid[grid.poi[&PoI::Campfire(homeland)]].index;
    if homeland_campfire != vertex {
        ret.push((homeland_campfire, &EdgeCost::ScrollOfEscape));
    }
    ret
}

pub fn find_path(
    grid: &MapGrid,
    homeland: Homeland,
    scroll_of_escape_cost: u32,
    from: CellIndex,
    to: CellIndex,
    (c1, c2): (CostComparator, CostComparator),
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
        for (edge_index, edge_cost) in inflight_edges(grid, homeland, lowest_cost_index) {
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
