use egui::load::{BytesLoadResult, BytesPoll};
use egui::{ScrollArea, Visuals};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct MarshrutkaApp {
    #[serde(skip)]
    raw_map: Option<BytesLoadResult>,
}

impl MarshrutkaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let mut result: MarshrutkaApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        egui_extras::install_image_loaders(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(Visuals::light());

        result.raw_map = Some(
            cc.egui_ctx
                .try_load_bytes("https://api.chatwars.me/webview/map"),
        );

        result
    }

    fn top_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();

                let is_web = cfg!(target_arch = "wasm32");
                ui.menu_button("File", |ui| {
                    // NOTE: no File->Quit on web pages
                    if !is_web {
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                bottom_frame(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Marshrutka");

            let s = match self.raw_map.as_ref().unwrap() {
                Ok(BytesPoll::Pending { .. }) => {
                    ctx.request_repaint();
                    Cow::Borrowed(include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/Map.html"
                    )))
                }
                Ok(BytesPoll::Ready { bytes, .. }) => String::from_utf8_lossy(bytes),
                Err(e) => Cow::Owned(e.to_string()),
            };

            ScrollArea::vertical().show(ui, |ui| {
                ui.label(s);
            });
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
