use crate::emoji::EmojiCode;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Deserialize, Serialize, Default, Eq, PartialEq, Copy, Clone)]
pub enum Homeland {
    #[default]
    Blue,
    Red,
    Green,
    Yellow,
}

impl Homeland {
    pub fn as_str(&self) -> &'static str {
        match self {
            Homeland::Blue => "Blue",
            Homeland::Red => "Red",
            Homeland::Green => "Green",
            Homeland::Yellow => "Yellow",
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
        self.as_str().fmt(f)
    }
}
