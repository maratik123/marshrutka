use crate::binary_heap::BinaryHeap;
use crate::cost::{
    CostComparator, EdgeCost, TotalCost, CARAVAN_CAMPFIRE_CENTER, CARAVAN_CENTER_ENEMY,
    CARAVAN_CENTER_HOMELAND, CARAVAN_FARLAND_HOMELAND, CARAVAN_HOMELAND_FARLAND,
    CARAVAN_HOMELAND_NEIGHBOUR, CARAVAN_NEIGHBOUR_FARLAND, CARAVAN_NEIGHBOUR_HOMELAND,
    CARAVAN_NEIGHBOUR_NEIGHBOUR,
};
use crate::grid::{MapGrid, PoI};
use crate::homeland::Homeland;
use crate::index::{Border, BorderDirection, CellIndex, Pos};
use smallvec::SmallVec;
use std::collections::HashMap;
use strum::IntoEnumIterator;

type EdgeCostRef<'a> = &'a EdgeCost;
type Edges<'a> = HashMap<CellIndex, HashMap<CellIndex, SmallVec<[EdgeCostRef<'a>; 4]>>>;

#[derive(Default, Debug)]
pub struct Graph {
    pub const_edges: Edges<'static>,
    pub dyn_edges: Edges<'static>,
}

macro_rules! append_edge {
    ($fn_name:ident, $fn_name_undirected:ident) => {
        fn $fn_name<'a>(
            edges: &mut Edges<'a>,
            from: CellIndex,
            to: CellIndex,
            cost: EdgeCostRef<'a>,
        ) {
            edges
                .entry(from)
                .or_default()
                .entry(to)
                .or_default()
                .push(cost);
        }

        fn $fn_name_undirected<'a>(
            edges: &mut Edges<'a>,
            from: CellIndex,
            to: CellIndex,
            cost: EdgeCostRef<'a>,
        ) {
            Self::$fn_name(edges, from, to, cost);
            Self::$fn_name(edges, to, from, cost);
        }
    };
}

impl Graph {
    pub fn new(grid: &MapGrid) -> Self {
        let grid_length = grid.grid.len();
        let mut const_edges = HashMap::with_capacity(grid_length);
        let homeland_square_size = (grid.square_size - 1) / 2;
        // process homelands
        for homeland in Homeland::iter() {
            for x in 1..=homeland_square_size {
                for y in 1..=homeland_square_size {
                    let from = CellIndex::Homeland {
                        homeland,
                        pos: (x, y).into(),
                    };
                    for d in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let to_pos_x = (x as isize + d.0) as usize;
                        let to_pos_y = (y as isize + d.1) as usize;
                        if (1..=homeland_square_size).contains(&to_pos_x)
                            && (1..=homeland_square_size).contains(&to_pos_y)
                        {
                            let to = CellIndex::Homeland {
                                homeland,
                                pos: (to_pos_x, to_pos_y).into(),
                            };
                            Self::append_edge_c(
                                &mut const_edges,
                                from,
                                to,
                                &EdgeCost::StandardMove,
                            );
                        }
                    }
                }
            }
        }
        // process borders
        for i in 1..=homeland_square_size {
            fn process_border(
                const_edges: &mut Edges,
                border: Border,
                homelands: [Homeland; 2],
                i: usize,
                pos: Pos,
            ) {
                let from = CellIndex::Border {
                    border,
                    shift: i as u8,
                };
                for homeland in homelands {
                    let to = CellIndex::Homeland { homeland, pos };
                    Graph::append_edge_undirected_c(const_edges, from, to, &EdgeCost::StandardMove);
                }
            }

            Border::iter().for_each(|border| {
                let pos = match border.direction() {
                    BorderDirection::Vertical => (1, i),
                    BorderDirection::Horizontal => (i, 1),
                }
                .into();
                process_border(&mut const_edges, border, border.neighbours(), i, pos);
            });
        }
        //process center
        for border in Border::iter() {
            let vertex = CellIndex::Border { border, shift: 1 };
            Self::append_edge_undirected_c(
                &mut const_edges,
                CellIndex::Center,
                vertex,
                &EdgeCost::CentralMove,
            );
        }
        // process caravans to center
        for homeland in Homeland::iter() {
            if let Some(i) = grid.poi.get(&PoI::Campfire(homeland)) {
                Self::append_edge_c(
                    &mut const_edges,
                    grid.grid[*i].index,
                    CellIndex::Center,
                    CARAVAN_CAMPFIRE_CENTER,
                );
            }
        }
        for val in const_edges.values_mut() {
            val.shrink_to_fit();
            for val in val.values_mut() {
                val.shrink_to_fit();
            }
        }
        Graph {
            const_edges,
            ..Graph::default()
        }
    }

    pub fn init_dynamic(&mut self, grid: &MapGrid, homeland: Homeland) {
        self.dyn_edges.clear();

        let [neighbour1, neighbour2] = homeland.neighbours().map(|neighbour| {
            grid.poi
                .get(&PoI::Campfire(neighbour))
                .map(|neighbour_i| grid.grid[*neighbour_i].index)
        });
        let farland = grid
            .poi
            .get(&PoI::Campfire(homeland.farland()))
            .map(|farland_i| grid.grid[*farland_i].index);

        if let Some(homeland) = grid
            .poi
            .get(&PoI::Campfire(homeland))
            .map(|homeland_i| grid.grid[*homeland_i].index)
        {
            // scroll of escape
            grid.grid
                .iter()
                .map(|cell| cell.index)
                .filter(|any_cell| any_cell != &homeland)
                .for_each(|any_cell| {
                    Self::append_edge_d(
                        &mut self.dyn_edges,
                        any_cell,
                        homeland,
                        &EdgeCost::ScrollOfEscape,
                    );
                });
            // caravans
            Self::append_edge_d(
                &mut self.dyn_edges,
                CellIndex::Center,
                homeland,
                CARAVAN_CENTER_HOMELAND,
            );

            [neighbour1, neighbour2]
                .into_iter()
                .flatten()
                .for_each(|neighbour| {
                    Self::append_edge_d(
                        &mut self.dyn_edges,
                        homeland,
                        neighbour,
                        CARAVAN_HOMELAND_NEIGHBOUR,
                    );
                    Self::append_edge_d(
                        &mut self.dyn_edges,
                        neighbour,
                        homeland,
                        CARAVAN_NEIGHBOUR_HOMELAND,
                    );
                });
            if let Some(farland) = farland {
                Self::append_edge_d(
                    &mut self.dyn_edges,
                    homeland,
                    farland,
                    CARAVAN_HOMELAND_FARLAND,
                );
                Self::append_edge_d(
                    &mut self.dyn_edges,
                    farland,
                    homeland,
                    CARAVAN_FARLAND_HOMELAND,
                );
            }
        }
        [neighbour1, neighbour2]
            .into_iter()
            .flatten()
            .for_each(|neighbour| {
                if let Some(farland) = farland {
                    Self::append_edge_undirected_d(
                        &mut self.dyn_edges,
                        neighbour,
                        farland,
                        CARAVAN_NEIGHBOUR_FARLAND,
                    );
                }
            });
        if let Some((neighbour1, neighbour2)) = neighbour1.zip(neighbour2) {
            Self::append_edge_undirected_d(
                &mut self.dyn_edges,
                neighbour1,
                neighbour2,
                CARAVAN_NEIGHBOUR_NEIGHBOUR,
            );
        }
        [neighbour1, neighbour2]
            .into_iter()
            .flatten()
            .chain(farland)
            .for_each(|enemy| {
                Self::append_edge_d(
                    &mut self.dyn_edges,
                    CellIndex::Center,
                    enemy,
                    CARAVAN_CENTER_ENEMY,
                );
            });

        self.dyn_edges.shrink_to_fit();
        for val in self.dyn_edges.values_mut() {
            val.shrink_to_fit();
            for val in val.values_mut() {
                val.shrink_to_fit();
            }
        }
    }

    pub fn find_path(
        &self,
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
            for (edge_index, edge_cost) in self
                .const_edges
                .get(&lowest_cost_index)
                .iter()
                .chain(self.dyn_edges.get(&lowest_cost_index).iter())
                .copied()
                .flatten()
                .flat_map(|(&next, next_cost)| {
                    next_cost.iter().map(move |&next_cost| (next, next_cost))
                })
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

    append_edge!(append_edge_c, append_edge_undirected_c);

    append_edge!(append_edge_d, append_edge_undirected_d);
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
