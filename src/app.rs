use crate::consts::{
    BLEACH_ALPHA, CELL_SIZE, DEFAULT_MAP_URL, FONT_CENTER, FONT_CENTER_SIZE, FONT_CORNER,
    FONT_CORNER_SIZE,
};
use crate::cost::{AggregatedCost, Command, CostComparator, TotalCost};
use crate::deep_link::{LINK_TO_SUPPORT_CHAT, send_command, send_command_to_bot};
use crate::emoji::EmojiMap;
use crate::grid::{MapGrid, MapGridResponse, arrow};
use crate::homeland::Homeland;
use crate::index::{CellIndex, CellIndexBuilder, CellIndexCommandSuffix, CellIndexLiteral};
use crate::pathfinder::FindPath;
use crate::translation::Translation;
use eframe::CreationContext;
use eframe::emath::Align;
use egui::emath::Rot2;
use egui::load::BytesPoll;
use egui::scroll_area::ScrollBarVisibility;
use egui::{
    Color32, FontId, Id, Image, ImageButton, InnerResponse, Layout, ScrollArea, TextBuffer,
    TextStyle, Ui, Visuals, Widget,
};
use rust_i18n::{set_locale, t};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::fmt::Display;
use std::iter;
use std::rc::Rc;
use strum::IntoEnumIterator;
use time::convert::{Hour, Second};
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
    scroll_of_escape_hq_cost: u32,
    hq_position: CellIndex,
    use_soe: bool,
    use_shq: bool,
    use_caravans: bool,
    arrive_at: Time,
    pause_between_steps: u32,
    #[serde(skip)]
    path: Option<Rc<TotalCost>>,
    map_url: String,
    command_via_chat_link: bool,
    route_guru_skill: u32,
    fleetfoot_skill: u32,
    translation: Translation,
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
                ui.menu_button(t!("file"), |ui| {
                    if ui.button(t!("settings")).clicked() {
                        self.show_settings = !self.show_settings;
                        self.need_to_save = true;
                        ui.close_menu();
                    }
                    // NOTE: no File->Quit on web pages
                    if !is_web {
                        ui.separator();
                        if ui.button(t!("quit")).clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            ui.close_menu();
                        }
                    }
                });
                if ui.button(t!("about")).clicked() {
                    self.show_about = !self.show_about;
                    self.need_to_save = true;
                }
                ui.separator();
                ui.label(t!("your_homeland"));
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
                        let mut sort_selector =
                            |ui: &mut Ui, salt, label: Cow<str>, val: &mut CostComparator| {
                                ui.label(label.as_str());
                                egui::ComboBox::from_id_salt(salt)
                                    .width(0.0)
                                    .selected_text(t!(val.as_str()))
                                    .show_ui(ui, |ui| {
                                        for sort in CostComparator::iter() {
                                            if ui
                                                .selectable_value(val, sort, t!(sort.as_str()))
                                                .changed()
                                            {
                                                self.need_to_save = true;
                                            }
                                        }
                                    })
                                    .response
                                    .on_hover_text(label);
                            };
                        sort_selector(ui, "sort_by", t!("sort_by"), &mut self.sort_by.0);
                        sort_selector(ui, "and_then_by", t!("and_then_by"), &mut self.sort_by.1);
                    });
            });
        });
    }

    fn about(&mut self, ctx: &egui::Context) {
        egui::Window::new(t!("about"))
            .id(Id::new("about"))
            .open(&mut self.show_about)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.heading("Marshrutka");
                    ui.add_space(8.0);
                    ui.label(t!("help_string_1"));
                    ui.add_space(8.0);
                    ui.label(t!("help1"));
                    ui.label(t!("help2"));
                    ui.add_space(4.0);
                    ui.label(t!("help_string_num"));
                    ui.add_space(8.0);
                    if cfg!(debug_assertions) {
                        egui::warn_if_debug_build(ui);
                        ui.add_space(4.0);
                    }
                    egui::Hyperlink::from_label_and_url(t!("support_chat"), LINK_TO_SUPPORT_CHAT)
                        .open_in_new_tab(true)
                        .ui(ui);
                    ui.add_space(4.0);
                    egui::Hyperlink::from_label_and_url(
                        t!("support_and_source"),
                        "https://github.com/maratik123/marshrutka",
                    )
                    .open_in_new_tab(true)
                    .ui(ui);
                });
            });
    }

    fn settings(&mut self, ctx: &egui::Context) {
        egui::Window::new(t!("settings"))
            .id(Id::new("settings"))
            .open(&mut self.show_settings)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    egui::ComboBox::new("select_your_homeland", t!("select_your_homeland"))
                        .selected_text(t!(self.homeland.name()))
                        .show_ui(ui, |ui| {
                            for homeland in Homeland::iter() {
                                if ui
                                    .selectable_value(
                                        &mut self.homeland,
                                        homeland,
                                        t!(homeland.name()),
                                    )
                                    .changed()
                                {
                                    self.need_to_save = true;
                                }
                            }
                        });
                    egui::ComboBox::new("language", t!("language"))
                        .selected_text(self.translation.name())
                        .show_ui(ui, |ui| {
                            for translation in Translation::iter() {
                                if ui
                                    .selectable_value(
                                        &mut self.translation,
                                        translation,
                                        translation.name(),
                                    )
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
                        ui.label(t!("route_guru_skill_level"));
                    });
                    ui.horizontal(|ui| {
                        if egui::DragValue::new(&mut self.fleetfoot_skill)
                            .clamp_existing_to_range(true)
                            .range(0..=1)
                            .ui(ui)
                            .changed()
                        {
                            self.need_to_save = true;
                        }
                        ui.label(t!("fleetfoot_skill_level"));
                    });
                    ui.horizontal(|ui| {
                        if egui::DragValue::new(&mut self.scroll_of_escape_cost)
                            .ui(ui)
                            .changed()
                        {
                            self.need_to_save = true;
                        }
                        ui.label(t!("scroll_of_escape_cost"));
                    });
                    ui.horizontal(|ui| {
                        if egui::DragValue::new(&mut self.scroll_of_escape_hq_cost)
                            .ui(ui)
                            .changed()
                        {
                            self.need_to_save = true;
                        }
                        ui.label(t!("scroll_of_escape_hq_cost"));
                    });
                    ui.horizontal(|ui| {
                        let literal: CellIndexLiteral = self.hq_position.into();
                        let literal_name: &'static str = literal.into();
                        egui::ComboBox::from_id_salt("HQ position literal")
                            .width(ui.spacing().combo_width / 2.5)
                            .selected_text(literal_name)
                            .show_ui(ui, |ui| {
                                for cell_index_literal in CellIndexLiteral::iter() {
                                    let cell_index_literal_name: &'static str =
                                        cell_index_literal.into();
                                    if ui
                                        .selectable_label(
                                            cell_index_literal == literal,
                                            cell_index_literal_name,
                                        )
                                        .clicked()
                                        && cell_index_literal != literal
                                    {
                                        self.hq_position =
                                            self.hq_position.mutate_by_literal(cell_index_literal);
                                        self.need_to_save = true;
                                    }
                                }
                            });
                        let max_val = self
                            .grid
                            .as_ref()
                            .map(|g| g.homeland_size() as u8)
                            .unwrap_or(1);
                        match &mut self.hq_position {
                            CellIndex::Center => {}
                            CellIndex::Homeland { pos, .. } => {
                                if egui::DragValue::new(&mut pos.x)
                                    .clamp_existing_to_range(true)
                                    .range(1..=max_val)
                                    .ui(ui)
                                    .changed()
                                {
                                    self.need_to_save = true;
                                }
                                ui.label("#");
                                if egui::DragValue::new(&mut pos.y)
                                    .clamp_existing_to_range(true)
                                    .range(1..=max_val)
                                    .ui(ui)
                                    .changed()
                                {
                                    self.need_to_save = true;
                                }
                            }
                            CellIndex::Border { shift, .. } => {
                                if egui::DragValue::new(shift)
                                    .clamp_existing_to_range(true)
                                    .range(1..=max_val)
                                    .ui(ui)
                                    .changed()
                                {
                                    self.need_to_save = true;
                                }
                            }
                        }
                        ui.label(t!("hq_position"));
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
                        ui.label(t!("pause_between_steps"));
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
                        ui.label(t!("map_url"));
                    });
                    if ui
                        .checkbox(&mut self.command_via_chat_link, t!("use_direct_chat_link"))
                        .on_hover_text(t!("use_with_caution_at_least_on_android"))
                        .changed()
                    {
                        self.need_to_save = true;
                    }
                });
            });
    }

    fn commands(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            egui::CollapsingHeader::new(t!("commands"))
                .id_salt("commands_header")
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

                                    ui.label(t!("arrive_at"));
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

                                egui::Grid::new("commands_grid")
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label(t!("command"));
                                        ui.label(t!("duration"));
                                        ui.label(t!("total_time"));
                                        ui.label(t!("schedule_at"));
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
                                                AggregatedCost::ScrollOfEscapeHQ { .. } => {
                                                    "/use_shq".to_string()
                                                }
                                            };
                                            egui::Hyperlink::from_label_and_url(
                                                &command_str,
                                                if self.command_via_chat_link {
                                                    send_command_to_bot(&command_str)
                                                } else {
                                                    send_command(&command_str)
                                                },
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
        });
    }

    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(t!(
                    "from_to",
                    from = self.from.map(|s| s.to_string()).unwrap_or_default(),
                    to = self.to.map(|s| s.to_string()).unwrap_or_default()
                ));
                if ui.checkbox(&mut self.use_soe, "SoE").changed() {
                    self.need_to_save = true;
                }
                if ui.checkbox(&mut self.use_shq, "SHQ").changed() {
                    self.need_to_save = true;
                }
                if ui
                    .checkbox(&mut self.use_caravans, t!("caravans"))
                    .changed()
                {
                    self.need_to_save = true;
                }
            });

            ui.separator();

            let grid_response = egui::CollapsingHeader::new(t!("map"))
                .id_salt("map")
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
                            ui.small(t!("hint", help1 = t!("help1"), help2 = t!("help2")));
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
                            AggregatedCost::ScrollOfEscapeHQ { .. } => Color32::WHITE,
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
                        ui.label(t!("loading"));
                    });
                });
                return false;
            }
            Ok(BytesPoll::Ready { bytes, .. }) => String::from_utf8_lossy(bytes),
            Err(e) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.vertical(|ui| {
                            ui.heading(t!("error"));
                            ui.label(e.to_string())
                        });
                    });
                });
                return false;
            }
        };
        match MapGrid::parse(s.as_ref()) {
            Ok(grid) => {
                self.hq_position = CellIndexBuilder::from(self.hq_position)
                    .clamp(grid.homeland_size() as u8)
                    .build();
                self.need_to_save = true;
                self.grid = Some(grid);
                true
            }
            Err(err) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        ui.heading(t!("invalid_map"));
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
                FindPath {
                    homeland: self.homeland,
                    scroll_of_escape_cost: self.scroll_of_escape_cost,
                    scroll_of_escape_hq_cost: self.scroll_of_escape_hq_cost,
                    use_soe: self.use_soe,
                    hq_position: if self.use_shq {
                        Some(self.hq_position)
                    } else {
                        None
                    },
                    use_caravans: self.use_caravans,
                    route_guru: self.route_guru_skill.into(),
                    fleetfoot: self.fleetfoot_skill.into(),
                    sort_by: self.sort_by,
                    grid: self.grid.as_ref().unwrap(),
                }
                .eval(from, to)
            })
            .map(Rc::new);
        self.path.is_some()
    }

    fn post_process(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.need_to_save {
            self.update_path();
            self.need_to_save = false;

            if let Some(storage) = frame.storage_mut() {
                self.save_app(storage);
            }

            ctx.request_repaint();
        } else if self.path.is_none() && self.update_path() {
            ctx.request_repaint();
        }
    }

    fn save_app(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

impl eframe::App for MarshrutkaApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        set_locale(self.translation.to_locale_name());

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
            show_settings: true,
            show_about: true,
            grid: Default::default(),
            from: Default::default(),
            to: Default::default(),
            homeland: Default::default(),
            need_to_save: Default::default(),
            sort_by: (CostComparator::Legs, CostComparator::Money),
            scroll_of_escape_cost: 24,
            scroll_of_escape_hq_cost: 75,
            hq_position: CellIndex::Center,
            use_soe: true,
            use_shq: false,
            use_caravans: true,
            arrive_at: Time::MIDNIGHT,
            pause_between_steps: Default::default(),
            path: Default::default(),
            map_url: DEFAULT_MAP_URL.to_string(),
            command_via_chat_link: Default::default(),
            route_guru_skill: Default::default(),
            fleetfoot_skill: Default::default(),
            translation: Translation::En,
        }
    }
}
