use crate::consts::{FONT_CENTER, FONT_CENTER_SIZE, FONT_CORNER, FONT_CORNER_SIZE};
use crate::cost::CostComparator;
use crate::emoji::EmojiMap;
use crate::grid::MapGrid;
use crate::homeland::Homeland;
use crate::index::CellIndex;
use crate::pathfinder::Graph;
use eframe::emath::Align;
use egui::load::BytesPoll;
use egui::{FontId, Image, ImageButton, Layout, ScrollArea, TextStyle, Visuals, Widget};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::sync::{Arc, RwLock};
use strum::IntoEnumIterator;

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct MarshrutkaApp {
    #[serde(skip)]
    emojis: OnceCell<EmojiMap>,
    show_settings: bool,
    show_about: bool,
    #[serde(skip)]
    grid: Option<Arc<MapGrid>>,
    from: Option<CellIndex>,
    to: Option<CellIndex>,
    homeland: Homeland,
    #[serde(skip)]
    need_to_save: bool,
    #[serde(skip)]
    graph: Option<Arc<RwLock<Graph>>>,
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

    fn emojis(&mut self, ctx: &egui::Context) -> &EmojiMap {
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
            });
        });
    }

    fn about(&mut self, ctx: &egui::Context) {
        egui::Window::new("About")
            .open(&mut self.show_about)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.heading("Marshrutka");
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

    fn eval_dyn_graph(&mut self) {
        let grid = self.grid.clone().unwrap();
        let graph = self.graph.clone().unwrap();
        {
            let mut graph = graph.write().unwrap();
            graph.init_dynamic(grid.as_ref(), self.homeland);
        }
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
                    self.grid = Some(Arc::new(grid));
                    self.graph = Some(Arc::new(RwLock::new(Graph::new(
                        self.grid.as_ref().unwrap().as_ref(),
                    ))));
                    self.eval_dyn_graph();
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

        let grid = self.grid.clone().unwrap();
        let graph = self.graph.clone().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Marshrutka");
            ui.label(format!(
                "From '{}' to '{}'",
                self.from.map(|s| s.to_string()).unwrap_or_default(),
                self.to.map(|s| s.to_string()).unwrap_or_default()
            ));

            ScrollArea::both().show(ui, |ui| {
                ui.collapsing("Map", |ui| {
                    let (from, to) = ScrollArea::both()
                        .show(ui, |ui| grid.ui_content(ui, self.emojis(ui.ctx())))
                        .inner;
                    if let Some(from) = from {
                        self.from = Some(from);
                        self.need_to_save = true;
                    }
                    if let Some(to) = to {
                        self.to = Some(to);
                        self.need_to_save = true;
                    }
                });
                {
                    let graph = graph.read().unwrap();
                    ui.separator();
                    if let Some((from, to)) = self.from.zip(self.to) {
                        ui.label(format!(
                            "{:#?}",
                            graph.find_path(
                                50,
                                from,
                                to,
                                (CostComparator::Money, CostComparator::Legs)
                            )
                        ));
                    }
                }
            });
        });

        if self.need_to_save {
            if let Some(storage) = frame.storage_mut() {
                self.save(storage);
            }
            self.need_to_save = false;
            self.eval_dyn_graph();
            ctx.request_repaint();
        }
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
