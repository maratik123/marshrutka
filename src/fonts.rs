use egui::load::Bytes;
use egui::{FontData, FontDefinitions, FontFamily, FontTweak};
use enum_map::{Enum, EnumMap};
use std::collections::BTreeMap;
use strum::EnumIter;

#[derive(EnumIter, Enum, Copy, Clone)]
pub enum Font {
    Hack,
    UbuntuLight,
    NotoEmojiRegular,
    EmojiIconFont,
}

impl Font {
    fn filename(&self) -> &'static str {
        match self {
            Font::Hack => "fonts/Hack-Regular.ttf",
            Font::UbuntuLight => "fonts/Ubuntu-Light.ttf",
            Font::NotoEmojiRegular => "fonts/NotoEmoji-Regular.ttf",
            Font::EmojiIconFont => "fonts/emoji-icon-font.ttf",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Font::Hack => "Hack",
            Font::UbuntuLight => "NotoEmoji-Regular",
            Font::NotoEmojiRegular => "NotoEmoji-Regular",
            Font::EmojiIconFont => "emoji-icon-font",
        }
    }

    fn tweak(&self) -> Option<FontTweak> {
        let scale = match self {
            Font::Hack | Font::UbuntuLight => return None,
            Font::NotoEmojiRegular => 0.81,
            Font::EmojiIconFont => 0.9,
        };
        Some(FontTweak {
            scale,
            ..FontTweak::default()
        })
    }

    fn font_families(&self) -> &'static [FontFamily] {
        match self {
            Font::Hack => &[FontFamily::Monospace],
            Font::UbuntuLight | Font::NotoEmojiRegular | Font::EmojiIconFont => {
                &[FontFamily::Proportional, FontFamily::Monospace]
            }
        }
    }

    pub fn font_definitions(loaded_fonts: EnumMap<Self, Bytes>) -> FontDefinitions {
        let families = loaded_fonts
            .iter()
            .flat_map(|(font, _)| {
                font.font_families()
                    .iter()
                    .map(move |family| (family.clone(), font.name().to_string()))
            })
            .fold(BTreeMap::<_, Vec<_>>::new(), |mut acc, (family, name)| {
                acc.entry(family).or_default().push(name);
                acc
            });

        let font_data = loaded_fonts
            .into_iter()
            .map(|(font, bytes)| {
                let name = font.name().to_string();
                let mut font_data = match bytes {
                    Bytes::Static(bytes) => FontData::from_static(bytes),
                    Bytes::Shared(bytes) => FontData::from_owned(bytes.to_vec()),
                };
                if let Some(tweak) = font.tweak() {
                    font_data = font_data.tweak(tweak);
                }
                (name, font_data)
            })
            .collect();

        FontDefinitions {
            families,
            font_data,
        }
    }
}
