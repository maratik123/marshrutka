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
            EmojiCode {
                c0: '\u{1f1ea}',
                c1: Some('\u{1f1fa}'),
            } => Homeland::Blue,
            EmojiCode {
                c0: '\u{1f1ee}',
                c1: Some('\u{1f1f2}'),
            } => Homeland::Red,
            EmojiCode {
                c0: '\u{1f1f2}',
                c1: Some('\u{1f1f4}'),
            } => Homeland::Green,
            EmojiCode {
                c0: '\u{1f1fb}',
                c1: Some('\u{1f1e6}'),
            } => Homeland::Yellow,
            _ => return Err(()),
        })
    }
}

impl Display for Homeland {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
