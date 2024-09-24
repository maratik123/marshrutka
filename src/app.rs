use crate::consts::{
    BLEACH_ALPHA, CELL_SIZE, FONT_CENTER, FONT_CENTER_SIZE, FONT_CORNER, FONT_CORNER_SIZE,
};
use crate::cost::{AggregatedCost, Command, CostComparator, TotalCost};
use crate::emoji::EmojiMap;
use crate::grid::{arrow, MapGrid, MapGridResponse};
use crate::homeland::Homeland;
use crate::index::{CellIndex, CellIndexCommandSuffix};
use crate::pathfinder::find_path;
use eframe::emath::Align;
use egui::emath::Rot2;
use egui::load::BytesPoll;
use egui::{
    Color32, FontId, Image, ImageButton, InnerResponse, Layout, ScrollArea, TextStyle, Ui, Visuals,
    Widget,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::fmt::Display;
use std::iter;
use strum::IntoEnumIterator;
use time::macros::format_description;
use time::{Duration, Time};

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
    scroll_of_escape_cost: u32,
    use_soe: bool,
    use_caravans: bool,
    actual: bool,
    arrive_at: Time,
    pause_between_steps: u32,
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
                if let Some(flag) = self.emojis(ui.ctx()).get_texture(emoji_code) {
                    if ImageButton::new(Image::new(&flag.corner).shrink_to_fit())
                        .ui(ui)
                        .clicked()
                    {
                        self.show_settings = true;
                    }
                }
                ui.separator();
                ScrollArea::horizontal().show(ui, |ui| {
                    ui.label("Sort by");
                    let mut sort_selector = |ui: &mut Ui, label, val: &mut CostComparator| {
                        egui::ComboBox::from_id_source(label)
                            .width(0.0)
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
                    ui.label("and then by");
                    sort_selector(ui, "Then sort by", &mut self.sort_by.1);
                });
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
                    ui.label("LMB/short tap - from, RMB/long tap - to");
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
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    egui::ComboBox::from_label("Select your homeland")
                        .selected_text(self.homeland.name())
                        .show_ui(ui, |ui| {
                            for homeland in Homeland::iter() {
                                if ui
                                    .selectable_value(&mut self.homeland, homeland, homeland.name())
                                    .changed()
                                {
                                    self.need_to_save = true;
                                }
                            }
                        });
                    ui.horizontal(|ui| {
                        if egui::DragValue::new(&mut self.scroll_of_escape_cost)
                            .ui(ui)
                            .changed()
                        {
                            self.need_to_save = true;
                        }
                        ui.label("Scroll of Escape cost");
                    });
                    ui.horizontal(|ui| {
                        if egui::DragValue::new(&mut self.pause_between_steps)
                            .clamp_to_range(true)
                            .range(0..=1000)
                            .ui(ui)
                            .changed()
                        {
                            self.need_to_save = true;
                        }
                        ui.label("Pause between steps (s)");
                    });
                });
            });
    }

    fn commands(&mut self, ctx: &egui::Context, path: &Option<TotalCost>) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let response = egui::CollapsingHeader::new("Commands")
                .default_open(true)
                .show(ui, |ui| {
                    if let Some(path) = path {
                        if !path.commands.is_empty()
                            && !matches!(
                                &path.commands[..],
                                [Command {
                                    aggregated_cost: AggregatedCost::NoMove,
                                    ..
                                }]
                            )
                        {
                            ScrollArea::horizontal().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    fn show_item(
                                        s: &MarshrutkaApp,
                                        ui: &mut Ui,
                                        ch: char,
                                        val: impl Display,
                                    ) {
                                        ui.scope(|ui| {
                                            ui.spacing_mut().item_spacing.x = 4.0;
                                            ui.label(val.to_string());
                                            Image::new(
                                                &s.emojis(ui.ctx())
                                                    .get_texture(&ch.into())
                                                    .unwrap()
                                                    .corner,
                                            )
                                            .max_height(ui.text_style_height(&TextStyle::Body))
                                            .ui(ui);
                                        });
                                    }
                                    show_item(self, ui, '\u{1f463}', path.legs);
                                    show_item(self, ui, '\u{23f0}', path.time);
                                    show_item(self, ui, '\u{1fa99}', path.money);
                                    ui.label("Arrive at:");

                                    let mut hr = self.arrive_at.hour();
                                    if egui::DragValue::new(&mut hr)
                                        .clamp_to_range(true)
                                        .range(0..=23)
                                        .ui(ui)
                                        .changed()
                                    {
                                        self.arrive_at = self.arrive_at.replace_hour(hr).unwrap();
                                        self.need_to_save = true;
                                    }
                                    ui.label("h");
                                    let mut mi = self.arrive_at.minute();
                                    if egui::DragValue::new(&mut mi)
                                        .clamp_to_range(true)
                                        .range(0..=59)
                                        .ui(ui)
                                        .changed()
                                    {
                                        self.arrive_at = self.arrive_at.replace_minute(mi).unwrap();
                                        self.need_to_save = true;
                                    }
                                    ui.label("m");
                                    let mut sec = self.arrive_at.second();
                                    if egui::DragValue::new(&mut sec)
                                        .clamp_to_range(true)
                                        .range(0..=59)
                                        .ui(ui)
                                        .changed()
                                    {
                                        self.arrive_at =
                                            self.arrive_at.replace_second(sec).unwrap();
                                        self.need_to_save = true;
                                    }
                                    ui.label("s");
                                });

                                egui::Grid::new("Commands").striped(true).show(ui, |ui| {
                                    ui.label("Command");
                                    ui.label("Duration");
                                    ui.label("Total time");
                                    ui.label("Schedule at");
                                    ui.end_row();
                                    let mut total_time = Duration::ZERO;
                                    let commands: Vec<_> = path
                                        .commands
                                        .iter()
                                        .filter(|command| {
                                            !matches!(
                                                command,
                                                Command {
                                                    aggregated_cost: AggregatedCost::NoMove,
                                                    ..
                                                }
                                            )
                                        })
                                        .copied()
                                        .collect();
                                    let pause_between_steps =
                                        Duration::seconds(self.pause_between_steps as i64);
                                    let times: Vec<_> = commands
                                        .iter()
                                        .rev()
                                        .scan(
                                            self.arrive_at + pause_between_steps,
                                            |acc, command| {
                                                *acc -= command.aggregated_cost.time()
                                                    + pause_between_steps;
                                                Some(*acc)
                                            },
                                        )
                                        .collect();
                                    let time_format =
                                        format_description!("[hour]:[minute]:[second]");
                                    for (command, time) in
                                        iter::zip(commands, times.into_iter().rev())
                                    {
                                        let command_str = match command.aggregated_cost {
                                            AggregatedCost::NoMove => continue,
                                            AggregatedCost::CentralMove { .. }
                                            | AggregatedCost::StandardMove { .. } => {
                                                format!(
                                                    "/go_direct_{}",
                                                    CellIndexCommandSuffix(command.to)
                                                )
                                            }
                                            AggregatedCost::Caravan { .. } => {
                                                format!(
                                                    "/car_{}",
                                                    CellIndexCommandSuffix(command.to)
                                                )
                                            }
                                            AggregatedCost::ScrollOfEscape { .. } => {
                                                "/use_soe".to_string()
                                            }
                                        };
                                        egui::Hyperlink::from_label_and_url(
                                            &command_str,
                                            format!("https://t.me/share/url?url={command_str}"),
                                        )
                                        .open_in_new_tab(true)
                                        .ui(ui);
                                        let command_time = command.aggregated_cost.time();
                                        ui.label(command_time.to_string());
                                        total_time += command_time;
                                        ui.label(total_time.to_string());
                                        ui.label(time.format(&time_format).unwrap());
                                        ui.end_row();
                                    }
                                });
                            });
                        }
                    }
                });
            if response.fully_closed() || response.fully_open() {
                self.need_to_save = true;
            }
        });
    }
}

impl eframe::App for MarshrutkaApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let bytes = ctx.try_load_bytes("https://api.chatwars.me/webview/map");

        let (s, actual) = match &bytes {
            Ok(BytesPoll::Pending { .. }) => {
                ctx.request_repaint();
                egui::CentralPanel::default().show(ctx, |ui| ui.label("Loading..."));
                return;
            }
            Ok(BytesPoll::Ready { bytes, .. }) => (String::from_utf8_lossy(bytes), true),
            Err(_) => (
                Cow::Borrowed(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/Map.html"
                ))),
                false,
            ),
        };

        if self.grid.is_none() {
            match MapGrid::parse(s.as_ref()) {
                Ok(grid) => {
                    self.grid = Some(grid);
                    self.actual = actual;
                }
                Err(err) => {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| ui.label(format!("Invalid map: {err}")));
                    return;
                }
            };
        }

        let path = {
            self.from.zip(self.to).and_then(|(from, to)| {
                find_path(
                    self.grid.as_ref().unwrap(),
                    self.homeland,
                    self.scroll_of_escape_cost,
                    (from, to),
                    self.sort_by,
                    self.use_soe,
                    self.use_caravans,
                )
            })
        };

        self.top_menu(ctx);

        self.commands(ctx, &path);

        self.settings(ctx);
        self.about(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.horizontal(|ui| {
                ui.label(format!(
                    "From '{}' to '{}'",
                    self.from.map(|s| s.to_string()).unwrap_or_default(),
                    self.to.map(|s| s.to_string()).unwrap_or_default()
                ));
                if ui.checkbox(&mut self.use_soe, "Use SoE").changed() {
                    self.need_to_save = true;
                }
                if ui
                    .checkbox(&mut self.use_caravans, "Use caravans")
                    .changed()
                {
                    self.need_to_save = true;
                }
            });

            ui.separator();

            let grid_response = egui::CollapsingHeader::new(if self.actual {
                "Map"
            } else {
                "Map (not actual)"
            })
            .default_open(true)
            .show(ui, |ui| {
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
            });
            if grid_response.fully_closed() || grid_response.fully_open() {
                self.need_to_save = true;
            }
            let grid_response = grid_response.body_returned;
            if let Some((path, (centers, grid_response))) = path.as_ref().zip(grid_response) {
                let painter = ui.painter_at(grid_response.interact_rect);
                let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
                let tip_length = CELL_SIZE / 4.0;
                for command in path.commands.iter() {
                    arrow(
                        &painter,
                        rot,
                        tip_length,
                        centers[&command.from],
                        centers[&command.to],
                        match command.aggregated_cost {
                            AggregatedCost::NoMove => continue,
                            AggregatedCost::CentralMove { .. } => Color32::RED,
                            AggregatedCost::StandardMove { .. } => Color32::BLUE,
                            AggregatedCost::Caravan { .. } => Color32::GREEN,
                            AggregatedCost::ScrollOfEscape { .. } => Color32::BROWN,
                        }
                        .gamma_multiply(BLEACH_ALPHA as f32 / 255.0),
                    );
                }
            }
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
            scroll_of_escape_cost: 50,
            use_soe: true,
            use_caravans: true,
            actual: Default::default(),
            arrive_at: Time::MIDNIGHT,
            pause_between_steps: Default::default(),
        }
    }
}
