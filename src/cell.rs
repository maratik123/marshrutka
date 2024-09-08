use crate::emoji::{EmojiCode, EmojiMap};
use egui::emath::RectTransform;
use egui::{Color32, Image, ImageSource, Label, Pos2, Rect, Sense, Ui, Vec2, Widget};

pub enum CellElement {
    Text(String),
    Emoji(EmojiCode),
}

pub struct Cell {
    pub color32: Color32,
    pub top_left: Option<CellElement>,
    pub top_right: Option<CellElement>,
    pub bottom_left: Option<CellElement>,
    pub bottom_right: Option<CellElement>,
    pub center: Option<CellElement>,
}

impl Cell {
    pub fn ui_content(&self, ui: &mut Ui, emoji_map: &EmojiMap) {
        let (response, painter) = ui.allocate_painter(Vec2::splat(90.0), Sense::click());

        let to_screen = RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.size()),
            response.rect,
        );
        if let Some(cell_element) = &self.top_left {
            match cell_element {
                CellElement::Emoji(emoji_code) => match emoji_map.get_texture(emoji_code) {
                    None => {
                        ui.label(emoji_code.to_string());
                    }
                    Some(texture) => {
                        ui.image(ImageSource::from(&texture.p16));
                    }
                },
                CellElement::Text(text) => {
                    ui.label(text);
                }
            }
        };
    }
}
