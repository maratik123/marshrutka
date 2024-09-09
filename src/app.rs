use crate::cell::{Cell, CellElement};
use crate::emoji::EmojiMap;
use crate::grid::{CELL_SIZE, MAP_SIZE};
use egui::load::BytesPoll;
use egui::{Color32, FontId, Grid, Label, ScrollArea, TextStyle, Vec2, Visuals};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::sync::{Arc, LazyLock};

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct MarshrutkaApp {
    #[serde(skip)]
    emojis: OnceCell<EmojiMap>,
    show_settings: bool,
}

pub const FONT32: &str = "32";
pub const FONT16: &str = "16";

impl MarshrutkaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let result: MarshrutkaApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        egui_extras::install_image_loaders(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(Visuals::light());
        let body_font_family = TextStyle::Body.resolve(&cc.egui_ctx.style()).family;
        let mut styles = (*cc.egui_ctx.style()).clone();

        styles.text_styles.insert(
            TextStyle::Name(FONT32.into()),
            FontId::new(32.0, body_font_family.clone()),
        );
        styles.text_styles.insert(
            TextStyle::Name(FONT16.into()),
            FontId::new(16.0, body_font_family),
        );
        cc.egui_ctx.set_style(styles);

        result
    }

    fn emojis(&mut self, ctx: &egui::Context) -> &EmojiMap {
        self.emojis.get_or_init(|| EmojiMap::new(ctx))
    }

    fn top_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();

                let is_web = cfg!(target_arch = "wasm32");
                ui.menu_button("File", |ui| {
                    if ui.button("Settings").clicked() {
                        self.show_settings ^= true;
                        ui.close_menu();
                    }
                    // NOTE: no File->Quit on web pages
                    if !is_web {
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            ui.close_menu();
                        }
                    }
                });
            });
        });
    }
}

impl eframe::App for MarshrutkaApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.top_menu(ctx);

        egui::Window::new("Settings")
            .open(&mut self.show_settings)
            .show(ctx, |ui| {
                ui.heading("Settings");
            });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                bottom_frame(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Marshrutka");
            ui.image(
                &self
                    .emojis(ui.ctx())
                    .get_texture(&'\u{1f33e}'.into())
                    .unwrap()
                    .p32,
            );
            ui.image(
                &self
                    .emojis(ui.ctx())
                    .get_texture(&'\u{1f33e}'.into())
                    .unwrap()
                    .p16,
            );

            let bytes = ui
                .ctx()
                .try_load_bytes("https://api.chatwars.me/webview/map");

            {
                let s = match bytes {
                    Ok(BytesPoll::Pending { .. }) => {
                        ui.ctx().request_repaint();
                        Cow::Borrowed("Loading...")
                    }
                    Ok(BytesPoll::Ready { bytes, .. }) => {
                        Cow::Owned(String::from_utf8_lossy(bytes.as_ref()).to_string())
                    }
                    Err(_) => Cow::Borrowed(include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/Map.html"
                    ))),
                };

                ScrollArea::both().show(ui, |ui| {
                    Grid::new("map_grid")
                        .striped(true)
                        .spacing(Vec2::splat(1.0))
                        .min_col_width(CELL_SIZE * 2.0)
                        .min_row_height(CELL_SIZE * 2.0)
                        .show(ui, |ui| {
                            for i in 0..MAP_SIZE {
                                for j in 0..MAP_SIZE {
                                    ScrollArea::both().id_source((i, j)).show(ui, |ui| {
                                        Cell {
                                            color32: Color32::PLACEHOLDER,
                                            top_left: Some(CellElement::Text("TL".to_string())),
                                            top_right: Some(CellElement::Emoji(
                                                ('\u{26fa}', '\u{fe0f}').into(),
                                            )),
                                            bottom_left: Some(CellElement::Text("BL".to_string())),
                                            bottom_right: Some(CellElement::Text("BR".to_string())),
                                            center: Some(CellElement::Emoji('\u{1f33e}'.into())),
                                        }
                                        .ui_content(ui, self.emojis(ui.ctx()));
                                    });
                                }
                                ui.end_row();
                            }
                        });
                });
            }
        });
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

fn bottom_frame(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("Powered by ");
            ui.hyperlink_to("egui", "https://github.com/emilk/egui");
            ui.label(" and ");
            ui.hyperlink_to(
                "eframe",
                "https://github.com/emilk/egui/tree/master/crates/eframe",
            );
            ui.label(".");
        });
        if cfg!(debug_assertions) {
            ui.separator();
            egui::warn_if_debug_build(ui);
        }
        ui.separator();
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.hyperlink_to("Source code", "https://github.com/maratik123/marshrutka");
        ui.label(".");
    });
}
