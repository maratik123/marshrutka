use crate::homeland::Homeland;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use strum::EnumIter;

#[derive(
    Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug, EnumIter, Ord, PartialOrd,
)]
pub enum Border {
    BR,
    RG,
    GY,
    YB,
}

pub enum BorderDirection {
    Vertical,
    Horizontal,
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

impl Border {
    pub fn as_str(&self) -> &'static str {
        match self {
            Border::BR => "BR",
            Border::RG => "RG",
            Border::GY => "GY",
            Border::YB => "YB",
        }
    }

    pub fn neighbours(&self) -> [Homeland; 2] {
        match self {
            Border::BR => [Homeland::Blue, Homeland::Red],
            Border::RG => [Homeland::Red, Homeland::Green],
            Border::GY => [Homeland::Green, Homeland::Yellow],
            Border::YB => [Homeland::Yellow, Homeland::Blue],
        }
    }

    pub fn direction(&self) -> BorderDirection {
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
    Some(CellIndex::Homeland {
        homeland: homeland.parse().ok()?,
        pos: pos.parse().ok()?,
    })
}

fn parse_as_border(border: &str, shift: &str) -> Option<CellIndex> {
    Some(CellIndex::Border {
        border: border.parse().ok()?,
        shift: shift.parse().ok()?,
    })
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

from_u_to_pos!(u16 u32 u64 u128 usize);