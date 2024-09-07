use egui::load::{BytesLoadResult, BytesPoll};
use egui::{ColorImage, ScrollArea, TextureHandle, TextureOptions, Visuals};
use resvg::tiny_skia::Pixmap;
use resvg::usvg::{Options, Transform, Tree};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::HashMap;

struct EmojiTextures {
    p32: TextureHandle,
    p16: TextureHandle,
}

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct MarshrutkaApp {
    #[serde(skip)]
    raw_map: Option<BytesLoadResult>,
    #[serde(skip)]
    emojis: OnceCell<HashMap<char, EmojiTextures>>,
    show_settings: bool,
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

    fn emojis(&mut self, ctx: &egui::Context) -> &HashMap<char, EmojiTextures> {
        self.emojis.get_or_init(|| init_emojis(ctx))
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

macro_rules! char_to_emoji_map {
    [$(($ch:expr, $path:expr)),* $(,)?] => {
        [$((
            $ch,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/images/", $path)).as_ref(),
        )),*]
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
    let mut pixmap = pixmap.as_mut();
    resvg::render(tree, Transform::identity(), &mut pixmap);
    let image = ColorImage::from_rgba_premultiplied(
        [pixmap.width() as _, pixmap.height() as _],
        pixmap.data_mut(),
    );
    ctx.load_texture(name, image, TextureOptions::default())
}

fn init_emojis(ctx: &egui::Context) -> HashMap<char, EmojiTextures> {
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
            EmojiTextures {
                p32: svg_to_texture(ctx, 32),
                p16: svg_to_texture(ctx, 16),
            },
        )
    })
    .collect()
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

            let s = match self.raw_map.as_ref().unwrap() {
                Ok(BytesPoll::Pending { .. }) => {
                    ui.ctx().request_repaint();
                    Cow::Borrowed(include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/Map.html"
                    )))
                }
                Ok(BytesPoll::Ready { bytes, .. }) => String::from_utf8_lossy(bytes),
                Err(e) => Cow::Owned(e.to_string()),
            };

            ScrollArea::both().show(ui, |ui| {
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
