use crate::emoji::EmojiCode;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use strum::EnumIter;

#[derive(
    Deserialize,
    Serialize,
    Default,
    Eq,
    PartialEq,
    Copy,
    Clone,
    Hash,
    Debug,
    EnumIter,
    Ord,
    PartialOrd,
)]
pub enum Homeland {
    #[default]
    Blue,
    Red,
    Green,
    Yellow,
}

impl Homeland {
    pub fn name(&self) -> &'static str {
        match self {
            Homeland::Blue => "Blue",
            Homeland::Red => "Red",
            Homeland::Green => "Green",
            Homeland::Yellow => "Yellow",
        }
    }

    pub fn as_abbrev(&self) -> char {
        match self {
            Homeland::Blue => 'B',
            Homeland::Red => 'R',
            Homeland::Green => 'G',
            Homeland::Yellow => 'Y',
        }
    }

    pub fn neighbours(&self) -> [Homeland; 2] {
        match self {
            Homeland::Blue => [Homeland::Yellow, Homeland::Red],
            Homeland::Red => [Homeland::Blue, Homeland::Green],
            Homeland::Green => [Homeland::Red, Homeland::Yellow],
            Homeland::Yellow => [Homeland::Blue, Homeland::Green],
        }
    }

    pub fn farland(&self) -> Homeland {
        match self {
            Homeland::Blue => Homeland::Green,
            Homeland::Red => Homeland::Yellow,
            Homeland::Green => Homeland::Blue,
            Homeland::Yellow => Homeland::Red,
        }
    }
}

impl TryFrom<EmojiCode> for Homeland {
    type Error = ();

    fn try_from(value: EmojiCode) -> Result<Self, Self::Error> {
        Ok(match value {
            EmojiCode('\u{1f1ea}', Some('\u{1f1fa}')) => Homeland::Blue,
            EmojiCode('\u{1f1ee}', Some('\u{1f1f2}')) => Homeland::Red,
            EmojiCode('\u{1f1f2}', Some('\u{1f1f4}')) => Homeland::Green,
            EmojiCode('\u{1f1fb}', Some('\u{1f1e6}')) => Homeland::Yellow,
            _ => return Err(()),
        })
    }
}

impl From<&Homeland> for EmojiCode {
    fn from(value: &Homeland) -> Self {
        (*value).into()
    }
}

impl From<Homeland> for EmojiCode {
    fn from(value: Homeland) -> Self {
        match value {
            Homeland::Blue => ('\u{1f1ea}', '\u{1f1fa}'),
            Homeland::Red => ('\u{1f1ee}', '\u{1f1f2}'),
            Homeland::Green => ('\u{1f1f2}', '\u{1f1f4}'),
            Homeland::Yellow => ('\u{1f1fb}', '\u{1f1e6}'),
        }
        .into()
    }
}
impl Display for Homeland {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.name().fmt(f)
    }
}

impl FromStr for Homeland {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "B" => Homeland::Blue,
            "R" => Homeland::Red,
            "G" => Homeland::Green,
            "Y" => Homeland::Yellow,
            _ => {
                return Err(());
            }
        })
    }
}

impl TryFrom<char> for Homeland {
    type Error = ();

    fn try_from(ch: char) -> Result<Self, Self::Error> {
        Ok(match ch {
            'B' => Homeland::Blue,
            'R' => Homeland::Red,
            'G' => Homeland::Green,
            'Y' => Homeland::Yellow,
            _ => {
                return Err(());
            }
        })
    }
}