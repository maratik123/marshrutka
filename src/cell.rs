use crate::consts::{
    BLEACH_ALPHA, CELL_MARGIN, CELL_ROUNDING, CELL_SIZE, FONT_CENTER, FONT_CORNER,
};
use crate::emoji::{EmojiCode, EmojiMap};
use crate::grid::PoI;
use crate::homeland::Homeland;
use crate::index::CellIndex;
use arrayvec::ArrayVec;
use egui::{
    Align2, Color32, Margin, Painter, Pos2, Rect, Sense, TextStyle, TextureHandle, Ui, Vec2,
};
use enum_map::EnumMap;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum CellElement {
    Text(String),
    Emoji(EmojiCode),
}

#[derive(Debug)]
pub struct Cell {
    pub bg_color: Option<Color32>,
    pub top_left: Option<CellElement>,
    pub top_right: Option<CellElement>,
    pub bottom_left: Option<CellElement>,
    pub bottom_right: Option<CellElement>,
    pub center: Option<CellElement>,
    pub index: CellIndex,
    pub poi: Option<PoI>,
    pub x: i8,
    pub y: i8,
    pub nearest_campfire: OnceCell<EnumMap<Homeland, Option<CellIndex>>>,
}

struct DrawAttrs {
    align: Align2,
    large: bool,
    rect: Rect,
    bleach: bool,
}

impl TryFrom<&str> for CellElement {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err(());
        }
        let emoji: ArrayVec<_, 3> = s.chars().take(3).collect();
        Ok(EmojiCode::try_from(emoji.as_ref())
            .map(Self::Emoji)
            .unwrap_or_else(|_| Self::Text(s.to_string())))
    }
}

impl Cell {
    pub fn ui_content(
        &self,
        ui: &mut Ui,
        emoji_map: &EmojiMap,
    ) -> (Pos2, Option<CellIndex>, Option<CellIndex>) {
        let (response, painter) = ui.allocate_painter(Vec2::splat(CELL_SIZE), Sense::click());
        let rect = response.rect - Margin::same(CELL_MARGIN);
        if let Some(bg_color) = self.bg_color {
            painter.rect_filled(response.rect, CELL_ROUNDING, bg_color);
        }

        for (cell_element, align, large, bleach) in [
            (&self.center, Align2::CENTER_CENTER, true, true),
            (&self.top_left, Align2::LEFT_TOP, false, false),
            (&self.top_right, Align2::RIGHT_TOP, false, false),
            (&self.bottom_left, Align2::LEFT_BOTTOM, false, false),
            (&self.bottom_right, Align2::RIGHT_BOTTOM, false, false),
        ] {
            self.draw_element(
                ui,
                &painter,
                emoji_map,
                cell_element,
                DrawAttrs {
                    align,
                    large,
                    rect,
                    bleach,
                },
            );
        }

        (
            rect.center(),
            if response.clicked() {
                Some(self.index)
            } else {
                None
            },
            if response.secondary_clicked() {
                Some(self.index)
            } else {
                None
            },
        )
    }

    fn draw_element(
        &self,
        ui: &Ui,
        painter: &Painter,
        emoji_map: &EmojiMap,
        cell_element: &Option<CellElement>,
        attrs: DrawAttrs,
    ) {
        if let Some(cell_element) = &cell_element {
            match cell_element {
                CellElement::Emoji(emoji_code) => match emoji_map.get_texture(emoji_code) {
                    None => {
                        self.draw_text(ui, painter, emoji_code, attrs);
                    }
                    Some(texture) => {
                        self.draw_emoji_image(painter, texture.get(attrs.large), attrs);
                    }
                },
                CellElement::Text(text) => {
                    self.draw_text(ui, painter, text, attrs);
                }
            }
        }
    }

    fn draw_text(&self, ui: &Ui, painter: &Painter, text: impl ToString, attrs: DrawAttrs) {
        let font_size = if attrs.large {
            FONT_CENTER
        } else {
            FONT_CORNER
        };
        painter.text(
            attrs.align.pos_in_rect(&attrs.rect),
            attrs.align,
            text,
            ui.style().text_styles[&TextStyle::Name(font_size.into())].clone(),
            Color32::from_rgba_unmultiplied(
                0x2c,
                0x3e,
                0x50,
                if attrs.bleach { BLEACH_ALPHA } else { 255 },
            ),
        );
    }

    fn draw_emoji_image(
        &self,
        painter: &Painter,
        (image, image_size): (&TextureHandle, Vec2),
        attrs: DrawAttrs,
    ) {
        let image_rect = attrs.align.align_size_within_rect(image_size, attrs.rect);
        painter.image(
            image.id(),
            image_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::from_rgba_unmultiplied(
                255,
                255,
                255,
                if attrs.bleach { BLEACH_ALPHA } else { 255 },
            ),
        );
    }

    pub fn distance(&self, other: &Cell) -> usize {
        let Cell {
            x: from_x,
            y: from_y,
            ..
        } = self;
        let Cell {
            x: to_x, y: to_y, ..
        } = other;
        manhattan_distance(
            (*from_x as isize, *from_y as isize),
            (*to_x as isize, *to_y as isize),
        )
    }
}

fn manhattan_distance((from_x, from_y): (isize, isize), (to_x, to_y): (isize, isize)) -> usize {
    from_x.abs_diff(to_x) + from_y.abs_diff(to_y)
}

pub fn cell_parts(center: &Option<CellElement>) -> Option<PoI> {
    Some(match center {
        Some(CellElement::Emoji(EmojiCode('\u{1f525}', None))) => PoI::Campfire,
        Some(CellElement::Emoji(EmojiCode('\u{26f2}', None)))
        | Some(CellElement::Emoji(EmojiCode('\u{26f2}', Some('\u{fe0f}')))) => PoI::Fountain,
        Some(CellElement::Emoji(EmojiCode('\u{1f3db}', None)))
        | Some(CellElement::Emoji(EmojiCode('\u{1F3DB}', Some('\u{fe0f}')))) => PoI::Forum,
        _ => return None,
    })
}

impl From<String> for CellElement {
    fn from(text: String) -> Self {
        CellElement::Text(text)
    }
}

impl From<EmojiCode> for CellElement {
    fn from(emoji_code: EmojiCode) -> Self {
        CellElement::Emoji(emoji_code)
    }
}

impl Display for CellElement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CellElement::Text(text) => text.fmt(f),
            CellElement::Emoji(emoji_code) => emoji_code.fmt(f),
        }
    }
}

impl<'a> From<&'a CellElement> for Cow<'a, str> {
    fn from(value: &'a CellElement) -> Self {
        match value {
            CellElement::Text(text) => Cow::Borrowed(text.as_str()),
            CellElement::Emoji(emoji_code) => Cow::Owned(emoji_code.to_string()),
        }
    }
}
