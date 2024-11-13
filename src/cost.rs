use crate::index::CellIndex;
use crate::skill::{Fleetfoot, Skill};
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign};
use strum::{EnumIter, IntoStaticStr};
use time::Duration;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum EdgeCost {
    NoMove,
    CentralMove,
    StandardMove,
    Caravan(CaravanCost),
    ScrollOfEscape,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub struct CaravanCost {
    pub time: Duration,
    pub money: u32,
}

struct ToFountainMove {
    time: Duration,
    from: CellIndex,
    to: CellIndex,
    fleetfoot: Fleetfoot,
}

impl EdgeCost {
    pub const fn legs(&self) -> u32 {
        match self {
            EdgeCost::NoMove
            | EdgeCost::CentralMove
            | EdgeCost::Caravan(_)
            | EdgeCost::ScrollOfEscape => 0,
            EdgeCost::StandardMove => 1,
        }
    }

    pub const fn money(&self, scroll_of_escape_cost: u32) -> u32 {
        match self {
            EdgeCost::NoMove | EdgeCost::StandardMove | EdgeCost::CentralMove => 0,
            EdgeCost::Caravan(CaravanCost { money, .. }) => *money,
            EdgeCost::ScrollOfEscape => scroll_of_escape_cost,
        }
    }

    pub const fn time(&self) -> Duration {
        match self {
            EdgeCost::StandardMove => Duration::minutes(3),
            EdgeCost::CentralMove => Duration::seconds(10),
            EdgeCost::Caravan(CaravanCost { time, .. }) => *time,
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

#[derive(Eq, PartialEq, Copy, Clone, Debug, Ord, PartialOrd)]
pub struct Command {
    pub aggregated_cost: AggregatedCost,
    pub from: CellIndex,
    pub to: CellIndex,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Ord, PartialOrd)]
pub enum AggregatedCost {
    NoMove,
    CentralMove {
        time: Duration,
    },
    StandardMove {
        time: Duration,
        legs: u32,
        fleetfoot: Fleetfoot,
    },
    Caravan(CaravanCost),
    ScrollOfEscape {
        money: u32,
    },
}

impl AggregatedCost {
    pub fn time(&self) -> Duration {
        match self {
            AggregatedCost::ScrollOfEscape { .. } | AggregatedCost::NoMove => Duration::ZERO,
            AggregatedCost::CentralMove { time }
            | AggregatedCost::Caravan(CaravanCost { time, .. }) => *time,
            AggregatedCost::StandardMove {
                time, fleetfoot, ..
            } => fleetfoot.time(*time).unwrap_or(*time),
        }
    }

    pub fn money(&self) -> u32 {
        match self {
            AggregatedCost::NoMove
            | AggregatedCost::CentralMove { .. }
            | AggregatedCost::StandardMove { .. } => 0,
            AggregatedCost::Caravan(CaravanCost { money, .. })
            | AggregatedCost::ScrollOfEscape { money, .. } => *money,
        }
    }

    pub fn legs(&self) -> u32 {
        match self {
            AggregatedCost::NoMove
            | AggregatedCost::CentralMove { .. }
            | AggregatedCost::Caravan(_)
            | AggregatedCost::ScrollOfEscape { .. } => 0,
            AggregatedCost::StandardMove { legs, .. } => *legs,
        }
    }
}

impl From<(EdgeCost, u32, Fleetfoot)> for AggregatedCost {
    fn from((edge_cost, scroll_of_escape_cost, fleetfoot): (EdgeCost, u32, Fleetfoot)) -> Self {
        match edge_cost {
            EdgeCost::NoMove => AggregatedCost::NoMove,
            EdgeCost::CentralMove => AggregatedCost::CentralMove {
                time: edge_cost.time(),
            },
            EdgeCost::StandardMove => AggregatedCost::StandardMove {
                legs: edge_cost.legs(),
                time: edge_cost.time(),
                fleetfoot,
            },
            EdgeCost::Caravan(caravan_cost) => AggregatedCost::Caravan(caravan_cost),
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
    pub commands: SmallVec<[Command; 5]>,
}

impl TotalCost {
    pub fn new(from: CellIndex) -> Self {
        Self {
            commands: smallvec![Command {
                aggregated_cost: AggregatedCost::NoMove,
                from,
                to: from,
            }],
            ..Self::default()
        }
    }
}

impl AddAssign<(EdgeCost, u32, Fleetfoot, CellIndex, CellIndex)> for TotalCost {
    fn add_assign(
        &mut self,
        (edge_cost, scroll_of_escape_cost, fleetfoot, from, to): (
            EdgeCost,
            u32,
            Fleetfoot,
            CellIndex,
            CellIndex,
        ),
    ) {
        let legs = edge_cost.legs();
        let money = edge_cost.money(scroll_of_escape_cost);
        let time = edge_cost.time();
        let (aggregated_cost, from) = match (self.commands.last(), edge_cost) {
            (
                Some(Command {
                    aggregated_cost: AggregatedCost::NoMove,
                    ..
                }),
                _,
            ) => (
                (edge_cost, scroll_of_escape_cost, fleetfoot).into(),
                self.commands.pop().unwrap().from,
            ),
            (
                Some(Command {
                    aggregated_cost:
                        AggregatedCost::StandardMove {
                            legs: agg_legs,
                            time: agg_time,
                            fleetfoot,
                        },
                    ..
                }),
                EdgeCost::StandardMove,
            ) => {
                let aggregated_cost = AggregatedCost::StandardMove {
                    legs: *agg_legs + legs,
                    time: *agg_time + time,
                    fleetfoot: *fleetfoot,
                };
                (aggregated_cost, self.commands.pop().unwrap().from)
            }
            _ => (
                match edge_cost {
                    EdgeCost::NoMove => AggregatedCost::NoMove,
                    EdgeCost::CentralMove => AggregatedCost::CentralMove { time },
                    EdgeCost::StandardMove => AggregatedCost::StandardMove {
                        time,
                        legs,
                        fleetfoot,
                    },
                    EdgeCost::Caravan(_) => AggregatedCost::Caravan(CaravanCost { time, money }),
                    EdgeCost::ScrollOfEscape => AggregatedCost::ScrollOfEscape { money },
                },
                from,
            ),
        };
        self.commands.push(Command {
            aggregated_cost,
            from,
            to,
        });
        (self.legs, self.money, self.time) = self
            .commands
            .iter()
            .map(|command| {
                let aggregated_cost = &command.aggregated_cost;
                (
                    aggregated_cost.legs(),
                    aggregated_cost.money(),
                    aggregated_cost.time(),
                )
            })
            .reduce(|(a_legs, a_money, a_time), (b_legs, b_money, b_time)| {
                (a_legs + b_legs, a_money + b_money, a_time + b_time)
            })
            .unwrap_or_default();
    }
}

impl AddAssign<&ToFountainMove> for TotalCost {
    fn add_assign(&mut self, fountain_move: &ToFountainMove) {
        self.time += fountain_move.time;
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
            _ => (fountain_move.from, Duration::ZERO),
        };
        self.commands.push(Command {
            aggregated_cost: if from == fountain_move.to {
                AggregatedCost::NoMove
            } else {
                AggregatedCost::StandardMove {
                    time: time + fountain_move.time,
                    legs: 0,
                    fleetfoot: fountain_move.fleetfoot,
                }
            },
            from,
            to: fountain_move.to,
        })
    }
}

impl Add<(EdgeCost, u32, Fleetfoot, CellIndex, CellIndex)> for &TotalCost {
    type Output = TotalCost;

    fn add(self, rhs: (EdgeCost, u32, Fleetfoot, CellIndex, CellIndex)) -> Self::Output {
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
