use crate::consts::{BLEACH_ALPHA, FONT_CENTER, FONT_CENTER_SIZE, FONT_CORNER, FONT_CORNER_SIZE};
use crate::cost::{CostComparator, EdgeCost};
use crate::emoji::EmojiMap;
use crate::grid::{arrow, MapGrid, MapGridResponse};
use crate::homeland::Homeland;
use crate::index::CellIndex;
use crate::pathfinder::find_path;
use eframe::emath::Align;
use egui::load::BytesPoll;
use egui::{
    Color32, FontId, Image, ImageButton, InnerResponse, Layout, ScrollArea, TextStyle, Ui, Visuals,
    Widget,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::OnceCell;
use strum::IntoEnumIterator;

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct MarshrutkaApp {
    #[serde(skip)]
    emojis: OnceCell<EmojiMap>,
    show_settings: bool,
    show_about: bool,
    #[serde(skip)]
    grid: Option<MapGrid>,
    from: Option<CellIndex>,
    to: Option<CellIndex>,
    homeland: Homeland,
    #[serde(skip)]
    need_to_save: bool,
    sort_by: (CostComparator, CostComparator),
}

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
            TextStyle::Name(FONT_CENTER.into()),
            FontId::new(FONT_CENTER_SIZE, body_font_family.clone()),
        );
        styles.text_styles.insert(
            TextStyle::Name(FONT_CORNER.into()),
            FontId::new(FONT_CORNER_SIZE, body_font_family),
        );
        styles.visuals = Visuals::dark();
        cc.egui_ctx.set_style(styles);

        result
    }

    fn emojis(&self, ctx: &'_ egui::Context) -> &EmojiMap {
        self.emojis.get_or_init(|| EmojiMap::new(ctx))
    }

    fn top_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                let is_web = cfg!(target_arch = "wasm32");
                ui.menu_button("File", |ui| {
                    if ui.button("Settings").clicked() {
                        self.show_settings ^= true;
                        self.need_to_save = true;
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
                if ui.button("About").clicked() {
                    self.show_about ^= true;
                    self.need_to_save = true;
                }
                ui.separator();
                ui.label("Your homeland: ");
                let emoji_code = &self.homeland.into();
                if let Some(flag) = self.emojis(ctx).get_texture(emoji_code) {
                    if ImageButton::new(Image::new(&flag.corner).shrink_to_fit())
                        .ui(ui)
                        .clicked()
                    {
                        self.show_settings = true;
                    }
                }
                ui.separator();
                ui.label("Preference: ");
                let mut sort_selector = |ui: &mut Ui, label, val: &mut CostComparator| {
                    egui::ComboBox::from_id_source(label)
                        .selected_text(val.as_str())
                        .show_ui(ui, |ui| {
                            for sort in CostComparator::iter() {
                                if ui.selectable_value(val, sort, sort.as_str()).changed() {
                                    self.need_to_save = true;
                                }
                            }
                        })
                        .response
                        .on_hover_text(label);
                };
                sort_selector(ui, "Sort by", &mut self.sort_by.0);
                sort_selector(ui, "Then sort by", &mut self.sort_by.1);
            });
        });
    }

    fn about(&mut self, ctx: &egui::Context) {
        egui::Window::new("About")
            .open(&mut self.show_about)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.heading("Marshrutka");
                    ui.add_space(8.0);
                    ui.label("Transport accessibility\nfor retarded people");
                    ui.add_space(8.0);
                    if cfg!(debug_assertions) {
                        egui::warn_if_debug_build(ui);
                    }
                    ui.hyperlink_to(
                        "Support and source code",
                        "https://github.com/maratik123/marshrutka",
                    );
                });
            });
    }

    fn settings(&mut self, ctx: &egui::Context) {
        egui::Window::new("Settings")
            .open(&mut self.show_settings)
            .show(ctx, |ui| {
                egui::Grid::new("Settings")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        egui::ComboBox::from_label("Select your homeland")
                            .selected_text(self.homeland.name())
                            .show_ui(ui, |ui| {
                                for homeland in Homeland::iter() {
                                    if ui
                                        .selectable_value(
                                            &mut self.homeland,
                                            homeland,
                                            homeland.name(),
                                        )
                                        .changed()
                                    {
                                        self.need_to_save = true;
                                    }
                                }
                            })
                    })
            });
    }
}

impl eframe::App for MarshrutkaApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let bytes = ctx.try_load_bytes("https://api.chatwars.me/webview/map");

        let s = match &bytes {
            Ok(BytesPoll::Pending { .. }) => {
                ctx.request_repaint();
                egui::CentralPanel::default().show(ctx, |ui| ui.label("Loading..."));
                return;
            }
            Ok(BytesPoll::Ready { bytes, .. }) => String::from_utf8_lossy(bytes),
            Err(_) => Cow::Borrowed(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/Map.html"
            ))),
        };

        if self.grid.is_none() {
            match MapGrid::parse(s.as_ref()) {
                Ok(grid) => {
                    self.grid = Some(grid);
                }
                Err(err) => {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| ui.label(format!("Invalid map: {err}")));
                    return;
                }
            };
        }

        self.top_menu(ctx);

        self.settings(ctx);
        self.about(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Marshrutka");
            ui.label(format!(
                "From '{}' to '{}'",
                self.from.map(|s| s.to_string()).unwrap_or_default(),
                self.to.map(|s| s.to_string()).unwrap_or_default()
            ));

            ScrollArea::both().show(ui, |ui| {
                let grid_response = ui
                    .collapsing("Map", |ui| {
                        let InnerResponse {
                            inner:
                                MapGridResponse {
                                    centers,
                                    left: from,
                                    right: to,
                                },
                            response,
                        } = ScrollArea::both()
                            .show(ui, |ui| {
                                let emojis = self.emojis(ui.ctx());
                                self.grid.as_ref().unwrap().ui_content(ui, emojis)
                            })
                            .inner;
                        if let Some(from) = from {
                            self.from = Some(from);
                            self.need_to_save = true;
                        }
                        if let Some(to) = to {
                            self.to = Some(to);
                            self.need_to_save = true;
                        }
                        (centers, response)
                    })
                    .body_returned;
                let path = {
                    ui.separator();
                    self.from.zip(self.to).and_then(|(from, to)| {
                        let path = find_path(
                            self.grid.as_ref().unwrap(),
                            self.homeland,
                            50,
                            from,
                            to,
                            self.sort_by,
                        );
                        ui.label(format!("{:#?}", path));
                        path
                    })
                };
                if let Some((path, (centers, grid_response))) = path.zip(grid_response) {
                    let painter = ui.painter_at(grid_response.interact_rect);
                    for command in path.commands {
                        arrow(
                            &painter,
                            centers[&command.from],
                            centers[&command.to],
                            match command.edge_cost {
                                EdgeCost::NoMove => continue,
                                EdgeCost::CentralMove => Color32::RED,
                                EdgeCost::StandardMove => Color32::BLUE,
                                EdgeCost::Caravan { .. } => Color32::GREEN,
                                EdgeCost::ScrollOfEscape => Color32::BROWN,
                            }
                            .gamma_multiply(BLEACH_ALPHA as f32 / 255.0),
                        );
                    }
                }
            });
        });

        if self.need_to_save {
            if let Some(storage) = frame.storage_mut() {
                self.save(storage);
            }
            self.need_to_save = false;
            ctx.request_repaint();
        }
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

impl Default for MarshrutkaApp {
    fn default() -> Self {
        Self {
            emojis: Default::default(),
            show_settings: Default::default(),
            show_about: Default::default(),
            grid: Default::default(),
            from: Default::default(),
            to: Default::default(),
            homeland: Default::default(),
            need_to_save: Default::default(),
            sort_by: (CostComparator::Legs, CostComparator::Money),
        }
    }
}
