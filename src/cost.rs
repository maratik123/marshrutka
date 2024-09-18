use crate::index::CellIndex;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Add, AddAssign};
use time::ext::NumericalDuration;
use time::Duration;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum EdgeCost {
    NoMove,
    CentralMove,
    StandardMove,
    Caravan { time: Duration, money: u32 },
    ScrollOfEscape,
}

impl EdgeCost {
    pub fn legs(&self) -> u32 {
        if matches!(self, EdgeCost::StandardMove) {
            1
        } else {
            0
        }
    }

    pub fn money(&self) -> MoneyCost {
        match self {
            EdgeCost::NoMove | EdgeCost::StandardMove | EdgeCost::CentralMove => MoneyCost::Fix(0),
            EdgeCost::Caravan { money, .. } => MoneyCost::Fix(*money),
            EdgeCost::ScrollOfEscape => MoneyCost::ScrollOfEscape,
        }
    }

    pub fn time(&self) -> Duration {
        match self {
            EdgeCost::StandardMove => 3.minutes(),
            EdgeCost::CentralMove => 10.seconds(),
            EdgeCost::Caravan { time, .. } => *time,
            EdgeCost::NoMove | EdgeCost::ScrollOfEscape => Duration::ZERO,
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum CostComparator {
    Legs,
    Time,
    Money,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
pub struct Command {
    pub edge_cost: &'static EdgeCost,
    pub from: CellIndex,
    pub to: CellIndex,
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
                edge_cost: &EdgeCost::NoMove,
                from,
                to: from,
            }],
            ..Self::default()
        }
    }
}

#[derive(Debug)]
pub enum MoneyCost {
    ScrollOfEscape,
    Fix(u32),
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
        self.legs += edge_cost.legs();
        self.money += match edge_cost.money() {
            MoneyCost::ScrollOfEscape => scroll_of_escape_cost,
            MoneyCost::Fix(money) => money,
        };
        self.time += edge_cost.time();
        let from = match (edge_cost, self.commands.last()) {
            (
                _,
                Some(Command {
                    edge_cost: EdgeCost::NoMove,
                    ..
                }),
            )
            | (
                EdgeCost::StandardMove,
                Some(Command {
                    edge_cost: EdgeCost::StandardMove,
                    ..
                }),
            ) => self.commands.pop().unwrap().from,
            _ => from,
        };
        self.commands.push(Command {
            edge_cost,
            from,
            to,
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

impl CostComparator {
    fn comparator(&self) -> impl Fn(&TotalCost, &TotalCost) -> Ordering {
        match self {
            CostComparator::Legs => |a: &TotalCost, b: &TotalCost| a.legs.cmp(&b.legs),
            CostComparator::Money => |a: &TotalCost, b: &TotalCost| a.money.cmp(&b.money),
            CostComparator::Time => |a: &TotalCost, b: &TotalCost| a.time.cmp(&b.time),
        }
    }

    fn probable_second_target(&self) -> CostComparator {
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

    pub fn as_str(&self) -> &str {
        match self {
            CostComparator::Legs => "Legs",
            CostComparator::Time => "Time",
            CostComparator::Money => "Money",
        }
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
