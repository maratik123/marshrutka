use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, EnumIter, IntoStaticStr)]
pub enum Translation {
    En,
    Es,
    Ru,
}

impl Translation {
    pub fn name(self) -> &'static str {
        match self {
            Translation::En => "English",
            Translation::Es => "Español",
            Translation::Ru => "Русский",
        }
    }

    pub fn to_locale_name(self) -> &'static str {
        match self {
            Translation::En => "en",
            Translation::Es => "es",
            Translation::Ru => "ru",
        }
    }
}
