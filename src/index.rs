use crate::homeland::Homeland;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use strum::{EnumCount, EnumIter, IntoStaticStr};

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Serialize,
    Deserialize,
    Debug,
    EnumIter,
    EnumCount,
    Ord,
    PartialOrd,
    IntoStaticStr,
)]
pub enum Border {
    BR,
    RG,
    GY,
    YB,
}

pub enum BorderDirection {
    Horizontal,
    Vertical,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug, Ord, PartialOrd)]
pub struct Pos {
    pub x: u8,
    pub y: u8,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug, Ord, PartialOrd)]
pub enum CellIndex {
    Center,
    Homeland { homeland: Homeland, pos: Pos },
    Border { border: Border, shift: u8 },
}

#[derive(
    Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug, Ord, PartialOrd, EnumIter,
)]
pub enum CellIndexLiteral {
    Center,
    Blue,
    Red,
    Green,
    Yellow,
    YB,
    BR,
    RG,
    GY,
}

impl From<CellIndex> for CellIndexLiteral {
    fn from(cell_index: CellIndex) -> Self {
        match cell_index {
            CellIndex::Center => CellIndexLiteral::Center,
            CellIndex::Homeland { homeland, .. } => homeland.into(),
            CellIndex::Border { border, .. } => border.into(),
        }
    }
}

impl From<Homeland> for CellIndexLiteral {
    fn from(value: Homeland) -> Self {
        match value {
            Homeland::Blue => CellIndexLiteral::Blue,
            Homeland::Red => CellIndexLiteral::Red,
            Homeland::Green => CellIndexLiteral::Green,
            Homeland::Yellow => CellIndexLiteral::Yellow,
        }
    }
}

impl From<Border> for CellIndexLiteral {
    fn from(value: Border) -> Self {
        match value {
            Border::BR => CellIndexLiteral::BR,
            Border::RG => CellIndexLiteral::RG,
            Border::GY => CellIndexLiteral::GY,
            Border::YB => CellIndexLiteral::YB,
        }
    }
}

impl From<CellIndexLiteral> for &'static str {
    fn from(value: CellIndexLiteral) -> Self {
        match value {
            CellIndexLiteral::Center => "0#0",
            CellIndexLiteral::Blue => "B",
            CellIndexLiteral::Red => "R",
            CellIndexLiteral::Green => "G",
            CellIndexLiteral::Yellow => "Y",
            CellIndexLiteral::YB => "YB",
            CellIndexLiteral::BR => "BR",
            CellIndexLiteral::RG => "RG",
            CellIndexLiteral::GY => "GY",
        }
    }
}

impl CellIndex {
    pub fn mutate_by_literal(self, to: CellIndexLiteral) -> CellIndex {
        match (self, to) {
            (_, CellIndexLiteral::Center) => CellIndexBuilder::Center,
            (CellIndex::Center, CellIndexLiteral::Blue) => CellIndexBuilder::Homeland {
                homeland: Homeland::Blue,
                pos: Pos { x: 1, y: 1 },
            },
            (CellIndex::Center, CellIndexLiteral::Red) => CellIndexBuilder::Homeland {
                homeland: Homeland::Red,
                pos: Pos { x: 1, y: 1 },
            },
            (CellIndex::Center, CellIndexLiteral::Green) => CellIndexBuilder::Homeland {
                homeland: Homeland::Green,
                pos: Pos { x: 1, y: 1 },
            },
            (CellIndex::Center, CellIndexLiteral::Yellow) => CellIndexBuilder::Homeland {
                homeland: Homeland::Yellow,
                pos: Pos { x: 1, y: 1 },
            },
            (CellIndex::Center, CellIndexLiteral::YB) => CellIndexBuilder::Border {
                border: Border::YB,
                shift: 1,
            },
            (CellIndex::Center, CellIndexLiteral::BR) => CellIndexBuilder::Border {
                border: Border::BR,
                shift: 1,
            },
            (CellIndex::Center, CellIndexLiteral::RG) => CellIndexBuilder::Border {
                border: Border::RG,
                shift: 1,
            },
            (CellIndex::Center, CellIndexLiteral::GY) => CellIndexBuilder::Border {
                border: Border::GY,
                shift: 1,
            },
            (CellIndex::Homeland { pos, .. }, CellIndexLiteral::Blue) => {
                CellIndexBuilder::Homeland {
                    homeland: Homeland::Blue,
                    pos,
                }
            }
            (CellIndex::Homeland { pos, .. }, CellIndexLiteral::Red) => {
                CellIndexBuilder::Homeland {
                    homeland: Homeland::Red,
                    pos,
                }
            }
            (CellIndex::Homeland { pos, .. }, CellIndexLiteral::Green) => {
                CellIndexBuilder::Homeland {
                    homeland: Homeland::Green,
                    pos,
                }
            }
            (CellIndex::Homeland { pos, .. }, CellIndexLiteral::Yellow) => {
                CellIndexBuilder::Homeland {
                    homeland: Homeland::Yellow,
                    pos,
                }
            }
            (CellIndex::Homeland { pos, .. }, CellIndexLiteral::YB) => CellIndexBuilder::Border {
                border: Border::YB,
                shift: pos.x,
            },
            (CellIndex::Homeland { pos, .. }, CellIndexLiteral::BR) => CellIndexBuilder::Border {
                border: Border::BR,
                shift: pos.x,
            },
            (CellIndex::Homeland { pos, .. }, CellIndexLiteral::RG) => CellIndexBuilder::Border {
                border: Border::RG,
                shift: pos.x,
            },
            (CellIndex::Homeland { pos, .. }, CellIndexLiteral::GY) => CellIndexBuilder::Border {
                border: Border::GY,
                shift: pos.x,
            },
            (CellIndex::Border { shift, .. }, CellIndexLiteral::Blue) => {
                CellIndexBuilder::Homeland {
                    homeland: Homeland::Blue,
                    pos: Pos { x: shift, y: 1 },
                }
            }
            (CellIndex::Border { shift, .. }, CellIndexLiteral::Red) => {
                CellIndexBuilder::Homeland {
                    homeland: Homeland::Red,
                    pos: Pos { x: shift, y: 1 },
                }
            }
            (CellIndex::Border { shift, .. }, CellIndexLiteral::Green) => {
                CellIndexBuilder::Homeland {
                    homeland: Homeland::Green,
                    pos: Pos { x: shift, y: 1 },
                }
            }
            (CellIndex::Border { shift, .. }, CellIndexLiteral::Yellow) => {
                CellIndexBuilder::Homeland {
                    homeland: Homeland::Yellow,
                    pos: Pos { x: shift, y: 1 },
                }
            }
            (CellIndex::Border { shift, .. }, CellIndexLiteral::YB) => CellIndexBuilder::Border {
                border: Border::YB,
                shift,
            },
            (CellIndex::Border { shift, .. }, CellIndexLiteral::BR) => CellIndexBuilder::Border {
                border: Border::BR,
                shift,
            },
            (CellIndex::Border { shift, .. }, CellIndexLiteral::RG) => CellIndexBuilder::Border {
                border: Border::RG,
                shift,
            },
            (CellIndex::Border { shift, .. }, CellIndexLiteral::GY) => CellIndexBuilder::Border {
                border: Border::GY,
                shift,
            },
        }
        .build()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug, Ord, PartialOrd)]
pub enum CellIndexBuilder {
    Center,
    Homeland { homeland: Homeland, pos: Pos },
    Border { border: Border, shift: u8 },
}

impl CellIndexBuilder {
    pub fn clamp(self, homeland_size: u8) -> Self {
        match self {
            CellIndexBuilder::Center => CellIndexBuilder::Center,
            CellIndexBuilder::Homeland { homeland, pos } => CellIndexBuilder::Homeland {
                homeland,
                pos: Pos {
                    x: pos.x.min(homeland_size),
                    y: pos.y.min(homeland_size),
                },
            },
            CellIndexBuilder::Border { border, shift } => CellIndexBuilder::Border {
                border,
                shift: shift.min(homeland_size),
            },
        }
    }

    pub const fn build(self) -> CellIndex {
        match self {
            CellIndexBuilder::Center
            | CellIndexBuilder::Homeland {
                pos: Pos { x: 0, y: 0 },
                ..
            }
            | CellIndexBuilder::Border { shift: 0, .. } => CellIndex::Center,
            CellIndexBuilder::Homeland {
                homeland: Homeland::Yellow,
                pos: Pos { x: 0, y },
            }
            | CellIndexBuilder::Homeland {
                homeland: Homeland::Blue,
                pos: Pos { x: 0, y },
            } => CellIndex::Border {
                border: Border::YB,
                shift: y,
            },
            CellIndexBuilder::Homeland {
                homeland: Homeland::Red,
                pos: Pos { x: 0, y },
            }
            | CellIndexBuilder::Homeland {
                homeland: Homeland::Green,
                pos: Pos { x: 0, y },
            } => CellIndex::Border {
                border: Border::RG,
                shift: y,
            },
            CellIndexBuilder::Homeland {
                homeland: Homeland::Blue,
                pos: Pos { x, y: 0 },
            }
            | CellIndexBuilder::Homeland {
                homeland: Homeland::Red,
                pos: Pos { x, y: 0 },
            } => CellIndex::Border {
                border: Border::BR,
                shift: x,
            },
            CellIndexBuilder::Homeland {
                homeland: Homeland::Green,
                pos: Pos { x, y: 0 },
            }
            | CellIndexBuilder::Homeland {
                homeland: Homeland::Yellow,
                pos: Pos { x, y: 0 },
            } => CellIndex::Border {
                border: Border::GY,
                shift: x,
            },
            CellIndexBuilder::Homeland { homeland, pos } => CellIndex::Homeland { homeland, pos },
            CellIndexBuilder::Border { border, shift } => CellIndex::Border { border, shift },
        }
    }
}

impl From<CellIndex> for CellIndexBuilder {
    fn from(cell_index: CellIndex) -> Self {
        match cell_index {
            CellIndex::Center => CellIndexBuilder::Center,
            CellIndex::Homeland { homeland, pos } => CellIndexBuilder::Homeland { homeland, pos },
            CellIndex::Border { border, shift } => CellIndexBuilder::Border { border, shift },
        }
    }
}

impl Border {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }

    pub const fn as_str_low(&self) -> &'static str {
        match self {
            Border::BR => "br",
            Border::RG => "rg",
            Border::GY => "gy",
            Border::YB => "yb",
        }
    }

    pub const fn neighbours(&self) -> [Homeland; 2] {
        match self {
            Border::BR => [Homeland::Blue, Homeland::Red],
            Border::RG => [Homeland::Red, Homeland::Green],
            Border::GY => [Homeland::Green, Homeland::Yellow],
            Border::YB => [Homeland::Yellow, Homeland::Blue],
        }
    }

    pub const fn direction(&self) -> BorderDirection {
        match self {
            Border::BR | Border::GY => BorderDirection::Horizontal,
            Border::RG | Border::YB => BorderDirection::Vertical,
        }
    }
}

impl Display for Border {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl Display for Pos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.x, self.y)
    }
}

impl Display for CellIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CellIndex::Center => "0#0".fmt(f),
            CellIndex::Homeland { homeland, pos } => write!(f, "{} {}", homeland.as_abbrev(), pos),
            CellIndex::Border { border, shift } => write!(f, "{} {}", border, shift),
        }
    }
}

pub struct CellIndexCommandSuffix(pub CellIndex);

impl Display for CellIndexCommandSuffix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            CellIndex::Center => <str as Display>::fmt("0_0", f),
            CellIndex::Homeland { homeland, pos } => {
                write!(f, "{}_{}_{}", homeland.as_abbrev_low(), pos.x, pos.y)
            }
            CellIndex::Border { border, shift } => write!(f, "{}_{shift}", border.as_str_low()),
        }
    }
}

impl FromStr for Border {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "BR" => Border::BR,
            "RG" => Border::RG,
            "GY" => Border::GY,
            "YB" => Border::YB,
            _ => {
                return Err(());
            }
        })
    }
}

impl FromStr for Pos {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (x, y) = s.split_once('#').ok_or(())?;
        let x = x.parse::<u8>().map_err(|_| ())?;
        let y = y.parse::<u8>().map_err(|_| ())?;
        Ok(Self { x, y })
    }
}

impl<'a, 'b> TryFrom<(Option<Cow<'a, str>>, Option<Cow<'b, str>>)> for CellIndex {
    type Error = ();

    fn try_from((a, b): (Option<Cow<'a, str>>, Option<Cow<'b, str>>)) -> Result<Self, Self::Error> {
        Ok(match (a, b) {
            (Some(a), Some(b)) => parse_as_homeland(&a, &b)
                .or_else(|| parse_as_border(&a, &b))
                .ok_or(())?,
            (None, Some(b)) if b == "0#0" => CellIndex::Center,
            _ => return Err(()),
        })
    }
}

impl FromStr for CellIndex {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s == "0#0" {
            Self::Center
        } else {
            let (left, right) = s.split_once(' ').ok_or(())?;
            parse_as_homeland(left, right)
                .or_else(|| parse_as_border(left, right))
                .ok_or(())?
        })
    }
}

fn parse_as_homeland(homeland: &str, pos: &str) -> Option<CellIndex> {
    Some(
        CellIndexBuilder::Homeland {
            homeland: homeland.parse().ok()?,
            pos: pos.parse().ok()?,
        }
        .build(),
    )
}

fn parse_as_border(border: &str, shift: &str) -> Option<CellIndex> {
    Some(
        CellIndexBuilder::Border {
            border: border.parse().ok()?,
            shift: shift.parse().ok()?,
        }
        .build(),
    )
}

macro_rules! from_u_to_pos {
    ($($t:ty)*) => {
        $(impl From<($t, $t)> for Pos {
            fn from((x, y): ($t, $t)) -> Self {
                Self {
                    x: x as u8,
                    y: y as u8,
                }
            }
        })*
    };
}

macro_rules! adjacent_pos {
    ($(($fn_name:ident, $t:ty)),* $(,)?) => {
        impl BorderDirection {
            $(pub fn $fn_name(&self, i: $t) -> Pos {
                match self {
                    BorderDirection::Horizontal => (i, 1),
                    BorderDirection::Vertical => (1, i),
                }.into()
            })*
        }
    }
}

from_u_to_pos!(u8 u16 u32 u64 usize);
adjacent_pos!(
    (adjacent_pos_u8, u8),
    (adjacent_pos_u16, u16),
    (adjacent_pos_u32, u32),
    (adjacent_pos_u64, u64),
    (adjacent_pos_usize, usize)
);
