use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use resvg::usvg::{Options, Transform, Tree};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Write};
use tiny_skia::Pixmap;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
pub struct EmojiCode {
    pub c0: char,
    pub c1: Option<char>,
}

pub enum EmojiContent {
    Alias(EmojiCode),
    Texture(EmojiTexture),
}

pub struct EmojiTexture {
    pub p32: TextureHandle,
    pub p16: TextureHandle,
}

pub struct EmojiMap(HashMap<EmojiCode, EmojiContent>);

impl EmojiMap {
    pub fn new(ctx: &egui::Context) -> Self {
        Self(init_emojis(ctx))
    }

    pub fn get_texture(&self, emoji_code: &EmojiCode) -> Option<&EmojiTexture> {
        let mut content = self.0.get(emoji_code)?;
        loop {
            match content {
                EmojiContent::Alias(emoji_code) => {
                    content = self.0.get(emoji_code)?;
                }
                EmojiContent::Texture(texture) => break Some(texture),
            }
        }
    }
}

fn svg_to_texture(
    ctx: &egui::Context,
    name: impl Into<String>,
    tree: &Tree,
    width: u32,
) -> TextureHandle {
    let size = tree.size().to_int_size().scale_to_width(width).unwrap();

    let mut pixmap = Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(tree, Transform::identity(), &mut pixmap.as_mut());
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

fn init_emojis(ctx: &egui::Context) -> HashMap<EmojiCode, EmojiContent> {
    // content
    char_to_emoji_map![
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
        ('\u{26f2}', "emoji_u26f2.svg"),
        ('\u{26fa}', "emoji_u26fa.svg"),
        ('\u{2728}', "emoji_u2728.svg"),
    ]
    .into_iter()
    .map(|(ch, content)| {
        let rtree = Tree::from_data(content, &Options::default()).unwrap();
        let svg_to_texture =
            |ctx, width| svg_to_texture(ctx, format!("{ch}|{width}"), &rtree, width);
        (
            ch,
            EmojiTexture {
                p32: svg_to_texture(ctx, 32),
                p16: svg_to_texture(ctx, 16),
            }
            .into(),
        )
    })
    .chain(
        // aliases
        [
            (
                EmojiCode::from(('\u{26fa}', '\u{fe0f}')),
                EmojiCode::from('\u{26fa}'),
            ),
            (
                EmojiCode::from(('\u{26f2}', '\u{fe0f}')),
                EmojiCode::from('\u{26f2}'),
            ),
        ]
        .into_iter()
        .map(|(from, to)| (from, to.into())),
    )
    .collect()
}

impl From<EmojiCode> for EmojiContent {
    fn from(emoji_code: EmojiCode) -> Self {
        Self::Alias(emoji_code)
    }
}

impl From<EmojiTexture> for EmojiContent {
    fn from(emoji_texture: EmojiTexture) -> Self {
        Self::Texture(emoji_texture)
    }
}

impl From<char> for EmojiCode {
    fn from(c0: char) -> Self {
        Self { c0, c1: None }
    }
}

impl From<(char, char)> for EmojiCode {
    fn from((c0, c1): (char, char)) -> Self {
        Self { c0, c1: Some(c1) }
    }
}

impl Display for EmojiCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char(self.c0)?;
        if let Some(c1) = self.c1 {
            f.write_char(c1)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
