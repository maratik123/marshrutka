use crate::app::{FONT16, FONT32};
use crate::emoji::{EmojiCode, EmojiMap};
use crate::grid::CELL_SIZE;
use egui::{
    Align2, Color32, Margin, Painter, Pos2, Rect, Sense, TextStyle, TextureHandle, Ui, Vec2,
};

pub enum CellElement {
    Text(String),
    Emoji(EmojiCode),
}

#[derive(Default)]
pub struct Cell {
    pub color32: Option<Color32>,
    pub top_left: Option<CellElement>,
    pub top_right: Option<CellElement>,
    pub bottom_left: Option<CellElement>,
    pub bottom_right: Option<CellElement>,
    pub center: Option<CellElement>,
}

struct DrawAttrs {
    align: Align2,
    large: bool,
    rect: Rect,
}

impl Cell {
    pub fn ui_content(&self, ui: &mut Ui, emoji_map: &EmojiMap) {
        let (response, painter) = ui.allocate_painter(Vec2::splat(CELL_SIZE), Sense::click());
        let rect = response.rect - Margin::same(8.0);
        if let Some(color32) = self.color32 {
            painter.rect_filled(response.rect, 5.0, color32);
        }

        self.draw_element(
            ui,
            &painter,
            emoji_map,
            &self.top_left,
            DrawAttrs {
                align: Align2::LEFT_TOP,
                large: false,
                rect,
            },
        );
        self.draw_element(
            ui,
            &painter,
            emoji_map,
            &self.top_right,
            DrawAttrs {
                align: Align2::RIGHT_TOP,
                large: false,
                rect,
            },
        );
        self.draw_element(
            ui,
            &painter,
            emoji_map,
            &self.bottom_left,
            DrawAttrs {
                align: Align2::LEFT_BOTTOM,
                large: false,
                rect,
            },
        );
        self.draw_element(
            ui,
            &painter,
            emoji_map,
            &self.bottom_right,
            DrawAttrs {
                align: Align2::RIGHT_BOTTOM,
                large: false,
                rect,
            },
        );
        self.draw_element(
            ui,
            &painter,
            emoji_map,
            &self.center,
            DrawAttrs {
                align: Align2::CENTER_CENTER,
                large: true,
                rect,
            },
        );
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
        let font_size = if attrs.large { FONT32 } else { FONT16 };
        painter.text(
            attrs.align.pos_in_rect(&attrs.rect),
            attrs.align,
            text,
            ui.style()
                .text_styles
                .get(&TextStyle::Name(font_size.into()))
                .unwrap()
                .clone(),
            ui.visuals().text_color(),
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
            Color32::WHITE,
        );
    }
}
