use crate::consts::{EMOJI_CORNER_SIZE, FONT_CENTER_SIZE};
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::Vec2;
use resvg::usvg::{Options, Transform, Tree};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use tiny_skia::{IntSize, Pixmap};

#[derive(Hash, PartialEq, Eq, Clone, Copy, Ord, PartialOrd, Debug)]
pub struct EmojiCode(pub char, pub Option<char>);

#[derive(Clone)]
pub struct EmojiTexture {
    pub center: TextureHandle,
    pub corner: TextureHandle,
}

pub struct EmojiMap(HashMap<EmojiCode, EmojiTexture>);

impl EmojiMap {
    pub fn new(ctx: &egui::Context) -> Self {
        Self(init_emojis(ctx))
    }

    pub fn get_texture(&self, emoji_code: &EmojiCode) -> Option<&EmojiTexture> {
        self.0.get(emoji_code)
    }
}

impl EmojiTexture {
    pub fn get(&self, large: bool) -> (&TextureHandle, Vec2) {
        if large {
            (&self.center, Vec2::splat(FONT_CENTER_SIZE))
        } else {
            (&self.corner, Vec2::splat(EMOJI_CORNER_SIZE))
        }
    }
}

fn svg_to_texture(
    ctx: &egui::Context,
    name: impl Into<String>,
    tree: &Tree,
    width: u32,
) -> TextureHandle {
    let svg_size = tree.size();
    let size = svg_size
        .to_int_size()
        .scale_to_width(width)
        .and_then(|s| s.scale_by(ctx.pixels_per_point()))
        .unwrap_or_else(|| IntSize::from_wh(width, width).unwrap());
    let transform = Transform::from_scale(
        size.width() as f32 / svg_size.width(),
        size.height() as f32 / svg_size.height(),
    );
    let mut pixmap = Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(tree, transform, &mut pixmap.as_mut());
    let image = ColorImage::from_rgba_premultiplied(
        [pixmap.width() as _, pixmap.height() as _],
        pixmap.data(),
    );
    ctx.load_texture(name, image, TextureOptions::default())
}

macro_rules! char_to_emoji_map {
    [$(($ch:expr, $path:expr)),* $(,)?] => {
        [$((
            EmojiCode::from($ch),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/images/", $path)).as_ref(),
        )),*]
    }
}

macro_rules! aliases_to_chars_map {
    [$(($alias:expr, $ch:expr)),* $(,)?] => {
        [$((
            EmojiCode::from($alias),
            EmojiCode::from($ch)
        )),*]
    }
}

fn init_emojis(ctx: &egui::Context) -> HashMap<EmojiCode, EmojiTexture> {
    let char_to_emoji_map = char_to_emoji_map![
        (('\u{1f1ea}', '\u{1f1fa}'), "emoji_u1f1ea_1f1fa.svg"),
        (('\u{1f1ee}', '\u{1f1f2}'), "emoji_u1f1ee_1f1f2.svg"),
        (('\u{1f1f2}', '\u{1f1f4}'), "emoji_u1f1f2_1f1f4.svg"),
        (('\u{1f1fb}', '\u{1f1e6}'), "emoji_u1f1fb_1f1e6.svg"),
        ('\u{1f332}', "emoji_u1f332.svg"),
        ('\u{1f333}', "emoji_u1f333.svg"),
        ('\u{1f33b}', "emoji_u1f33b.svg"),
        ('\u{1f33e}', "emoji_u1f33e.svg"),
        ('\u{1f344}', "emoji_u1f344.svg"),
        ('\u{1f347}', "emoji_u1f347.svg"),
        ('\u{1f34f}', "emoji_u1f34f.svg"),
        ('\u{1f356}', "emoji_u1f356.svg"),
        ('\u{1f3d4}', "emoji_u1f3d4.svg"),
        ('\u{1f3db}', "emoji_u1f3db.svg"),
        ('\u{1f3df}', "emoji_u1f3df.svg"),
        ('\u{1f3f0}', "emoji_u1f3f0.svg"),
        ('\u{1f410}', "emoji_u1f410.svg"),
        ('\u{1f411}', "emoji_u1f411.svg"),
        ('\u{1f414}', "emoji_u1f414.svg"),
        ('\u{1f417}', "emoji_u1f417.svg"),
        ('\u{1f41f}', "emoji_u1f41f.svg"),
        ('\u{1f48e}', "emoji_u1f48e.svg"),
        ('\u{1f525}', "emoji_u1f525.svg"),
        ('\u{1f573}', "emoji_u1f573.svg"),
        ('\u{1f578}', "emoji_u1f578.svg"),
        ('\u{1f5fc}', "emoji_u1f5fc.svg"),
        ('\u{1f5ff}', "emoji_u1f5ff.svg"),
        ('\u{1f6d6}', "emoji_u1f6d6.svg"),
        ('\u{1f6e1}', "emoji_u1f6e1.svg"),
        ('\u{1f987}', "emoji_u1f987.svg"),
        ('\u{1f98b}', "emoji_u1f98b.svg"),
        ('\u{1f98c}', "emoji_u1f98c.svg"),
        ('\u{1f9f1}', "emoji_u1f9f1.svg"),
        ('\u{1faa8}', "emoji_u1faa8.svg"),
        ('\u{1fab5}', "emoji_u1fab5.svg"),
        ('\u{2694}', "emoji_u2694.svg"),
        ('\u{26f2}', "emoji_u26f2.svg"),
        ('\u{26fa}', "emoji_u26fa.svg"),
        ('\u{2728}', "emoji_u2728.svg"),
    ];
    let aliases = aliases_to_chars_map![
        (('\u{1f6e1}', '\u{fe0f}'), '\u{1f6e1}'),
        (('\u{2694}', '\u{fe0f}'), '\u{2694}'),
        (('\u{26fa}', '\u{fe0f}'), '\u{26fa}'),
        (('\u{26f2}', '\u{fe0f}'), '\u{26f2}'),
    ];

    let mut map = HashMap::with_capacity(char_to_emoji_map.len() + aliases.len());

    map.extend(char_to_emoji_map.into_iter().map(|(ch, content)| {
        let rtree = Tree::from_data(content, &Options::default()).unwrap();
        let svg_to_texture =
            |ctx, width| svg_to_texture(ctx, format!("{ch}|{width}"), &rtree, width);
        (
            ch,
            EmojiTexture {
                center: svg_to_texture(ctx, FONT_CENTER_SIZE as _),
                corner: svg_to_texture(ctx, EMOJI_CORNER_SIZE as _),
            },
        )
    }));

    let aliases = aliases.map(|(from, to)| (from, map[&to].clone()));
    map.extend(aliases);

    map
}

impl From<char> for EmojiCode {
    fn from(c0: char) -> Self {
        Self(c0, None)
    }
}

impl From<(char, char)> for EmojiCode {
    fn from((c0, c1): (char, char)) -> Self {
        Self(c0, Some(c1))
    }
}

impl TryFrom<&[char]> for EmojiCode {
    type Error = ();

    fn try_from(value: &[char]) -> Result<Self, Self::Error> {
        Ok(match value {
            [c0] => (*c0).into(),
            [c0, c1] => (*c0, *c1).into(),
            _ => Err(())?,
        })
    }
}

impl Display for EmojiCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <char as Display>::fmt(&self.0, f)?;
        if let Some(c1) = self.1 {
            <char as Display>::fmt(&c1, f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Context;

    #[test]
    fn emoji_code_2_display() {
        let emoji_code = EmojiCode::from(('\u{26f2}', '\u{fe0f}'));
        let emoji_str = emoji_code.to_string();
        assert_eq!(emoji_str, "\u{26f2}\u{fe0f}");
    }

    #[test]
    fn emoji_code_display() {
        let emoji_code = EmojiCode::from('\u{26f2}');
        let emoji_str = emoji_code.to_string();
        assert_eq!(emoji_str, "\u{26f2}");
    }

    #[test]
    fn init_emojis_get() {
        let ctx = Context::default();
        let emojis = init_emojis(&ctx);
        assert_eq!(
            emojis[&('\u{1f6e1}', '\u{fe0f}').into()].center.id(),
            emojis[&'\u{1f6e1}'.into()].center.id()
        );
    }
}
