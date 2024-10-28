use crate::consts::{
    BLEACH_ALPHA, CELL_SIZE, DEFAULT_MAP_URL, FONT_CENTER, FONT_CENTER_SIZE, FONT_CORNER,
    FONT_CORNER_SIZE,
};
use crate::cost::{AggregatedCost, Command, CostComparator, TotalCost};
use crate::deep_link::{send_command_to_bot, LINK_TO_SUPPORT_CHAT};
use crate::emoji::EmojiMap;
use crate::grid::{arrow, MapGrid, MapGridResponse};
use crate::homeland::Homeland;
use crate::index::{CellIndex, CellIndexCommandSuffix};
use crate::pathfinder::{find_path, FindPathSettings};
use eframe::emath::Align;
use eframe::CreationContext;
use egui::emath::Rot2;
use egui::load::BytesPoll;
use egui::scroll_area::ScrollBarVisibility;
use egui::{
    Color32, FontId, Image, ImageButton, InnerResponse, Layout, ScrollArea, TextStyle, Ui, Visuals,
    Widget,
};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::cell::OnceCell;
use std::fmt::Display;
use std::iter;
use std::rc::Rc;
use strum::IntoEnumIterator;
use time::convert::{Hour, Second};
use time::macros::format_description;
use time::{Duration, Time};

const HELP1: &str = "To point \"From\" - LMB or short tap,";
const HELP2: &str = "to point \"To\" - RMB or long tap";

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
    arrive_at: Time,
    pause_between_steps: u32,
    #[serde(skip)]
    path: Option<Rc<TotalCost>>,
    map_url: String,
    route_guru_skill: u32,
}

impl MarshrutkaApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
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
        cc.egui_ctx.all_styles_mut(|styles| {
            let body_font_family = TextStyle::Body.resolve(styles).family;
            styles.text_styles.insert(
                TextStyle::Name(FONT_CENTER.into()),
                FontId::new(FONT_CENTER_SIZE, body_font_family.clone()),
            );
            styles.text_styles.insert(
                TextStyle::Name(FONT_CORNER.into()),
                FontId::new(FONT_CORNER_SIZE, body_font_family),
            );
            styles.visuals = Visuals::dark();
        });

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
                        self.show_settings = !self.show_settings;
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
                    self.show_about = !self.show_about;
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
                ScrollArea::horizontal()
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        let mut sort_selector = |ui: &mut Ui, label, val: &mut CostComparator| {
                            ui.label(label);
                            egui::ComboBox::from_id_salt(label)
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
                        sort_selector(ui, "and then by", &mut self.sort_by.1);
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
                    ui.label(HELP1);
                    ui.label(HELP2);
                    ui.add_space(4.0);
                    ui.label("Any numeric values can be\nchanged by dragging the number");
                    ui.add_space(8.0);
                    if cfg!(debug_assertions) {
                        egui::warn_if_debug_build(ui);
                        ui.add_space(4.0);
                    }
                    egui::Hyperlink::from_label_and_url("Support chat", LINK_TO_SUPPORT_CHAT)
                        .open_in_new_tab(true)
                        .ui(ui);
                    ui.add_space(4.0);
                    egui::Hyperlink::from_label_and_url(
                        "Support and source code",
                        "https://github.com/maratik123/marshrutka",
                    )
                    .open_in_new_tab(true)
                    .ui(ui);
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
                        if egui::DragValue::new(&mut self.route_guru_skill)
                            .clamp_existing_to_range(true)
                            .range(0..=1)
                            .ui(ui)
                            .changed()
                        {
                            self.need_to_save = true;
                        }
                        ui.label("Route Guru skill level");
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
                            .clamp_existing_to_range(true)
                            .range(0..=Second::per(Hour))
                            .ui(ui)
                            .changed()
                        {
                            self.need_to_save = true;
                        }
                        ui.label("Pause between steps (s)");
                    });
                    ui.horizontal(|ui| {
                        ui.scope(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            if egui::TextEdit::singleline(&mut self.map_url)
                                .desired_width(160.0)
                                .ui(ui)
                                .changed()
                            {
                                self.need_to_save = true;
                            }
                            if ui.button("↻").clicked() {
                                self.map_url = DEFAULT_MAP_URL.to_string();
                                self.need_to_save = true;
                            }
                        });
                        ui.label("Map URL");
                    });
                });
            });
    }

    fn commands(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            let response = egui::CollapsingHeader::new("Commands")
                .default_open(true)
                .show(ui, |ui| {
                    if let Some(path) = &self.path {
                        if !path.commands.is_empty()
                            && !matches!(
                                &path.commands[..],
                                [Command {
                                    aggregated_cost: AggregatedCost::NoMove,
                                    ..
                                }]
                            )
                        {
                            let path = path.clone();
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
                                    ui.scope(|ui| {
                                        ui.spacing_mut().item_spacing.x = 1.0;
                                        let formatter = |n, _| {
                                            let n = n as u32;
                                            format!("{n:02}")
                                        };
                                        let mut hr = self.arrive_at.hour();
                                        if egui::DragValue::new(&mut hr)
                                            .custom_formatter(formatter)
                                            .range(0..=23)
                                            .ui(ui)
                                            .changed()
                                        {
                                            self.arrive_at =
                                                self.arrive_at.replace_hour(hr).unwrap();
                                            self.need_to_save = true;
                                        }
                                        ui.label(":");
                                        let mut mi = self.arrive_at.minute();
                                        if egui::DragValue::new(&mut mi)
                                            .custom_formatter(formatter)
                                            .range(0..=59)
                                            .ui(ui)
                                            .changed()
                                        {
                                            self.arrive_at =
                                                self.arrive_at.replace_minute(mi).unwrap();
                                            self.need_to_save = true;
                                        }
                                        ui.label(":");
                                        let mut sec = self.arrive_at.second();
                                        if egui::DragValue::new(&mut sec)
                                            .custom_formatter(formatter)
                                            .range(0..=59)
                                            .ui(ui)
                                            .changed()
                                        {
                                            self.arrive_at =
                                                self.arrive_at.replace_second(sec).unwrap();
                                            self.need_to_save = true;
                                        }
                                    });
                                });

                                egui::Grid::new("Commands").striped(true).show(ui, |ui| {
                                    ui.label("Command");
                                    ui.label("Duration");
                                    ui.label("Total time");
                                    ui.label("Schedule at");
                                    ui.end_row();
                                    let mut total_time = Duration::ZERO;
                                    let commands: SmallVec<[_; 5]> = path
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
                                    let times: SmallVec<[_; 5]> = commands
                                        .iter()
                                        .rev()
                                        .scan(self.arrive_at, |acc, command| {
                                            *acc -= command.aggregated_cost.time()
                                                + pause_between_steps;
                                            Some(*acc)
                                        })
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
                                            AggregatedCost::Caravan(_) => {
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
                                            send_command_to_bot(&command_str),
                                        )
                                        .open_in_new_tab(true)
                                        .ui(ui);
                                        let command_time = command.aggregated_cost.time();
                                        ui.label(command_time.to_string());
                                        total_time += command_time + pause_between_steps;
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

    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "From '{}' to '{}'",
                    self.from.map(|s| s.to_string()).unwrap_or_default(),
                    self.to.map(|s| s.to_string()).unwrap_or_default()
                ));
                if ui.checkbox(&mut self.use_soe, "SoE").changed() {
                    self.need_to_save = true;
                }
                if ui.checkbox(&mut self.use_caravans, "Caravans").changed() {
                    self.need_to_save = true;
                }
            });

            ui.separator();

            let grid_response = egui::CollapsingHeader::new("Map")
                .id_salt("Map")
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
                            ui.small(format!("Hint: {HELP1} {HELP2}"));
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
            if let Some((path, (centers, grid_response))) = self.path.as_ref().zip(grid_response) {
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
                            AggregatedCost::Caravan(_) => Color32::DARK_GREEN,
                            AggregatedCost::ScrollOfEscape { .. } => Color32::BROWN,
                        }
                        .gamma_multiply(BLEACH_ALPHA as f32 / 255.0),
                    );
                }
            }
        });
    }

    fn load_map(&mut self, ctx: &egui::Context) -> bool {
        if self.grid.is_some() {
            return true;
        }
        let bytes = ctx.try_load_bytes(self.map_url.as_str());

        let s = match &bytes {
            Ok(BytesPoll::Pending { .. }) => {
                ctx.request_repaint();
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label("Loading...");
                    });
                });
                return false;
            }
            Ok(BytesPoll::Ready { bytes, .. }) => String::from_utf8_lossy(bytes),
            Err(e) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.vertical(|ui| {
                            ui.heading("Error:");
                            ui.label(e.to_string())
                        });
                    });
                });
                return false;
            }
        };
        match MapGrid::parse(s.as_ref()) {
            Ok(grid) => {
                self.grid = Some(grid);
                true
            }
            Err(err) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.heading("Invalid map");
                        ui.label(err.to_string());
                    });
                });
                false
            }
        }
    }

    fn update_path(&mut self) -> bool {
        self.path = self
            .from
            .zip(self.to)
            .and_then(|(from, to)| {
                find_path(
                    (from, to),
                    FindPathSettings {
                        homeland: self.homeland,
                        scroll_of_escape_cost: self.scroll_of_escape_cost,
                        use_soe: self.use_soe,
                        use_caravans: self.use_caravans,
                        route_guru: self.route_guru_skill.into(),
                        sort_by: self.sort_by,
                        grid: self.grid.as_ref().unwrap(),
                    },
                )
            })
            .map(Rc::new);
        self.path.is_some()
    }

    fn post_process(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.need_to_save {
            if self.path.is_none() && self.update_path() {
                ctx.request_repaint();
            }
            return;
        }

        self.update_path();
        self.need_to_save = false;

        if let Some(storage) = frame.storage_mut() {
            self.save_app(storage);
        }

        ctx.request_repaint();
    }

    fn save_app(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

impl eframe::App for MarshrutkaApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Prepare data
        if !self.load_map(ctx) {
            return;
        }

        // Side panels
        self.top_menu(ctx);
        self.commands(ctx);

        // Windows
        self.settings(ctx);
        self.about(ctx);

        // Central panel. Should be added after all other panels
        self.central_panel(ctx);

        // Postprocess state
        self.post_process(ctx, frame);
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.save_app(storage);
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
            scroll_of_escape_cost: 24,
            use_soe: true,
            use_caravans: true,
            arrive_at: Time::MIDNIGHT,
            pause_between_steps: Default::default(),
            path: Default::default(),
            map_url: DEFAULT_MAP_URL.to_string(),
            route_guru_skill: Default::default(),
        }
    }
}
