use time::Duration;

pub const DEFAULT_MAP_URL: &str = "https://maratik.fyi/api/chatwars/webview/map";

pub const FONT_CENTER: &str = "center";
pub const FONT_CENTER_SIZE: f32 = 32.0;

pub const FONT_CORNER: &str = "corner";
pub const FONT_CORNER_SIZE: f32 = 12.0;
pub const EMOJI_CORNER_SIZE: f32 = 16.0;

pub const GRID_SPACING: f32 = 2.0;

pub const CELL_SIZE: f32 = 62.0;
pub const CELL_MARGIN: i8 = 4;
pub const CELL_ROUNDING: f32 = 5.0;

pub const BLEACH_ALPHA: u8 = 166;

pub const ARROW_WIDTH: f32 = 5.0;
pub const ARROW_TIP_CIRCLE: f32 = 5.0;

pub const CARAVAN_TIME: Duration = Duration::minutes(4);
pub const CARAVAN_TO_HOME_MONEY: u32 = 2;
pub const CARAVAN_TO_CENTER_MONEY: u32 = 2;
pub const CARAVAN_MONEY: u32 = 5;
