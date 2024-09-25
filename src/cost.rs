use crate::index::CellIndex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign};
use strum::{EnumIter, IntoStaticStr};
use time::Duration;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum EdgeCost {
    NoMove,
    CentralMove,
    StandardMove,
    Caravan { time: Duration, money: u32 },
    ScrollOfEscape,
}

pub type EdgeCostRef = &'static EdgeCost;

struct ToFountainMove {
    time: Duration,
    from: CellIndex,
    to: CellIndex,
}

impl EdgeCost {
    pub const fn legs(&self) -> u32 {
        match self {
            EdgeCost::NoMove
            | EdgeCost::CentralMove
            | EdgeCost::Caravan { .. }
            | EdgeCost::ScrollOfEscape => 0,
            EdgeCost::StandardMove => 1,
        }
    }

    pub const fn money(&self, scroll_of_escape_cost: u32) -> u32 {
        match self {
            EdgeCost::NoMove | EdgeCost::StandardMove | EdgeCost::CentralMove => 0,
            EdgeCost::Caravan { money, .. } => *money,
            EdgeCost::ScrollOfEscape => scroll_of_escape_cost,
        }
    }

    pub const fn time(&self) -> Duration {
        match self {
            EdgeCost::StandardMove => Duration::minutes(3),
            EdgeCost::CentralMove => Duration::seconds(10),
            EdgeCost::Caravan { time, .. } => *time,
            EdgeCost::NoMove | EdgeCost::ScrollOfEscape => Duration::ZERO,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize, EnumIter, IntoStaticStr)]
pub enum CostComparator {
    Legs,
    Time,
    Money,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
pub struct Command {
    pub aggregated_cost: AggregatedCost,
    pub from: CellIndex,
    pub to: CellIndex,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
pub enum AggregatedCost {
    NoMove,
    CentralMove { time: Duration },
    StandardMove { time: Duration, legs: u32 },
    Caravan { time: Duration, money: u32 },
    ScrollOfEscape { money: u32 },
}

impl AggregatedCost {
    pub fn time(&self) -> Duration {
        match self {
            AggregatedCost::ScrollOfEscape { .. } | AggregatedCost::NoMove => Duration::ZERO,
            AggregatedCost::CentralMove { time }
            | AggregatedCost::StandardMove { time, .. }
            | AggregatedCost::Caravan { time, .. } => *time,
        }
    }

    pub fn money(&self) -> u32 {
        match self {
            AggregatedCost::NoMove
            | AggregatedCost::CentralMove { .. }
            | AggregatedCost::StandardMove { .. } => 0,
            AggregatedCost::Caravan { money, .. }
            | AggregatedCost::ScrollOfEscape { money, .. } => *money,
        }
    }

    pub fn legs(&self) -> u32 {
        match self {
            AggregatedCost::NoMove
            | AggregatedCost::CentralMove { .. }
            | AggregatedCost::Caravan { .. }
            | AggregatedCost::ScrollOfEscape { .. } => 0,
            AggregatedCost::StandardMove { legs, .. } => *legs,
        }
    }
}

impl From<(&EdgeCost, u32)> for AggregatedCost {
    fn from((edge_cost, scroll_of_escape_cost): (&EdgeCost, u32)) -> Self {
        match edge_cost {
            EdgeCost::NoMove => AggregatedCost::NoMove,
            EdgeCost::CentralMove => AggregatedCost::CentralMove {
                time: edge_cost.time(),
            },
            EdgeCost::StandardMove => AggregatedCost::StandardMove {
                legs: edge_cost.legs(),
                time: edge_cost.time(),
            },
            EdgeCost::Caravan { money, .. } => AggregatedCost::Caravan {
                time: edge_cost.time(),
                money: *money,
            },
            EdgeCost::ScrollOfEscape => AggregatedCost::ScrollOfEscape {
                money: scroll_of_escape_cost,
            },
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct TotalCost {
    pub legs: u32,
    pub money: u32,
    pub time: Duration,
    pub commands: Vec<Command>,
}

impl TotalCost {
    pub fn new(from: CellIndex) -> Self {
        Self {
            commands: vec![Command {
                aggregated_cost: AggregatedCost::NoMove,
                from,
                to: from,
            }],
            ..Self::default()
        }
    }
}

const CARAVAN_12_24: EdgeCost = EdgeCost::Caravan {
    money: 12,
    time: Duration::minutes(24),
};
const CARAVAN_30_24: EdgeCost = EdgeCost::Caravan {
    money: 30,
    time: Duration::minutes(24),
};
const CARAVAN_24_48: EdgeCost = EdgeCost::Caravan {
    money: 24,
    time: Duration::minutes(48),
};
const CARAVAN_60_48: EdgeCost = EdgeCost::Caravan {
    money: 60,
    time: Duration::minutes(48),
};

// HOMELAND - homeland campfire
// NEIGHBOUR - neighbour campfire
// FARLAND - far land campfire
// ENEMY - non-homeland campfire (neighbour or farland)
// CAMPFIRE - any campfire (homeland or enemy)
// CENTER - forum
pub const CARAVAN_CAMPFIRE_CENTER: &EdgeCost = &CARAVAN_12_24;
pub const CARAVAN_CENTER_HOMELAND: &EdgeCost = &CARAVAN_12_24;
pub const CARAVAN_CENTER_ENEMY: &EdgeCost = &CARAVAN_30_24;
pub const CARAVAN_NEIGHBOUR_HOMELAND: &EdgeCost = &CARAVAN_12_24;
pub const CARAVAN_HOMELAND_NEIGHBOUR: &EdgeCost = &CARAVAN_30_24;
pub const CARAVAN_HOMELAND_FARLAND: &EdgeCost = &CARAVAN_60_48;
pub const CARAVAN_FARLAND_HOMELAND: &EdgeCost = &CARAVAN_24_48;
pub const CARAVAN_NEIGHBOUR_FARLAND: &EdgeCost = &CARAVAN_30_24;
pub const CARAVAN_FARLAND_NEIGHBOUR: &EdgeCost = &CARAVAN_30_24;
pub const CARAVAN_NEIGHBOUR_NEIGHBOUR: &EdgeCost = &CARAVAN_60_48;

impl TotalCost {
    pub fn time_as_ms(&self) -> String {
        self.time.to_string()
    }
}

impl AddAssign<(&'static EdgeCost, u32, CellIndex, CellIndex)> for TotalCost {
    fn add_assign(
        &mut self,
        (edge_cost, scroll_of_escape_cost, from, to): (
            &'static EdgeCost,
            u32,
            CellIndex,
            CellIndex,
        ),
    ) {
        let legs = edge_cost.legs();
        let money = edge_cost.money(scroll_of_escape_cost);
        let time = edge_cost.time();
        self.legs += legs;
        self.money += money;
        self.time += time;
        let (aggregated_cost, from) = match (self.commands.last(), edge_cost) {
            (
                Some(Command {
                    aggregated_cost: AggregatedCost::NoMove,
                    ..
                }),
                edge_cost,
            ) => (
                (edge_cost, scroll_of_escape_cost).into(),
                self.commands.pop().unwrap().from,
            ),
            (
                Some(Command {
                    aggregated_cost: AggregatedCost::StandardMove { legs, time },
                    ..
                }),
                EdgeCost::StandardMove,
            ) => {
                let aggregated_cost = AggregatedCost::StandardMove {
                    legs: *legs + edge_cost.legs(),
                    time: *time + edge_cost.time(),
                };
                (aggregated_cost, self.commands.pop().unwrap().from)
            }
            (_, edge_cost) => (
                match edge_cost {
                    EdgeCost::NoMove => AggregatedCost::NoMove,
                    EdgeCost::CentralMove => AggregatedCost::CentralMove {
                        time: edge_cost.time(),
                    },
                    EdgeCost::StandardMove => AggregatedCost::StandardMove {
                        time: edge_cost.time(),
                        legs: edge_cost.legs(),
                    },
                    EdgeCost::Caravan { .. } => AggregatedCost::Caravan {
                        time: edge_cost.time(),
                        money: edge_cost.money(scroll_of_escape_cost),
                    },
                    EdgeCost::ScrollOfEscape => AggregatedCost::ScrollOfEscape {
                        money: edge_cost.money(scroll_of_escape_cost),
                    },
                },
                from,
            ),
        };
        self.commands.push(Command {
            aggregated_cost,
            from,
            to,
        })
    }
}

impl AddAssign<&ToFountainMove> for TotalCost {
    fn add_assign(&mut self, rhs: &ToFountainMove) {
        self.time += rhs.time;
        let (from, time) = match self.commands.last() {
            Some(Command {
                aggregated_cost: AggregatedCost::NoMove,
                ..
            }) => (self.commands.pop().unwrap().from, Duration::ZERO),
            Some(Command {
                aggregated_cost: AggregatedCost::StandardMove { time, .. },
                ..
            }) => {
                let time = *time;
                (self.commands.pop().unwrap().from, time)
            }
            _ => (rhs.from, Duration::ZERO),
        };
        self.commands.push(Command {
            aggregated_cost: if from == rhs.to {
                AggregatedCost::NoMove
            } else {
                AggregatedCost::StandardMove {
                    time: time + rhs.time,
                    legs: 0,
                }
            },
            from,
            to: rhs.to,
        })
    }
}

impl Add<(&'static EdgeCost, u32, CellIndex, CellIndex)> for &TotalCost {
    type Output = TotalCost;

    fn add(self, rhs: (&'static EdgeCost, u32, CellIndex, CellIndex)) -> Self::Output {
        let mut ret = self.clone();
        ret += rhs;
        ret
    }
}

impl Add<&ToFountainMove> for &TotalCost {
    type Output = TotalCost;

    fn add(self, rhs: &ToFountainMove) -> Self::Output {
        let mut ret = self.clone();
        ret += rhs;
        ret
    }
}

impl CostComparator {
    const fn comparator(&self) -> impl Fn(&TotalCost, &TotalCost) -> Ordering {
        match self {
            CostComparator::Legs => |a: &TotalCost, b: &TotalCost| a.legs.cmp(&b.legs),
            CostComparator::Money => |a: &TotalCost, b: &TotalCost| a.money.cmp(&b.money),
            CostComparator::Time => |a: &TotalCost, b: &TotalCost| a.time.cmp(&b.time),
        }
    }

    const fn probable_second_target(&self) -> CostComparator {
        match self {
            CostComparator::Legs => CostComparator::Time,
            CostComparator::Time => CostComparator::Legs,
            CostComparator::Money => CostComparator::Legs,
        }
    }

    fn eval_next(&self, c: CostComparator) -> (CostComparator, CostComparator) {
        let c = if self == &c {
            c.probable_second_target()
        } else {
            c
        };
        (
            c,
            match (self, c) {
                (CostComparator::Legs, CostComparator::Time) => CostComparator::Money,
                (CostComparator::Legs, CostComparator::Money) => CostComparator::Time,
                (CostComparator::Time, CostComparator::Legs) => CostComparator::Money,
                (CostComparator::Time, CostComparator::Money) => CostComparator::Legs,
                (CostComparator::Money, CostComparator::Legs) => CostComparator::Time,
                (CostComparator::Money, CostComparator::Time) => CostComparator::Legs,
                (_, _) => unreachable!("Can not choose last one comparator for case ({self}, {c})"),
            },
        )
    }

    pub fn as_str(&self) -> &'static str {
        self.into()
    }

    pub fn and_then(&self, c2: CostComparator) -> impl Fn(&TotalCost, &TotalCost) -> Ordering {
        let (c2, c3) = self.eval_next(c2);
        let c1 = self.comparator();
        let c2 = c2.comparator();
        let c3 = c3.comparator();
        move |t1, t2| -> Ordering {
            c1(t1, t2)
                .then_with(|| c2(t1, t2))
                .then_with(|| c3(t1, t2))
                .then_with(|| t1.commands.len().cmp(&t2.commands.len()))
                .then_with(|| t1.commands.iter().cmp(t2.commands.iter()))
        }
    }
}

impl Display for CostComparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
