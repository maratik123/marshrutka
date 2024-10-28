use std::fmt::Display;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;

    pub const LINK_TO_SUPPORT_CHAT: &str = "https://t.me/marshrutka_support";

    pub fn send_command_to_bot(command: impl Display) -> String {
        format!("https://t.me/ChatWarsBot?text={command}")
    }
}
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;

    pub const LINK_TO_SUPPORT_CHAT: &str = "tg://resolve?domain=marshrutka_support";

    pub fn send_command_to_bot(command: impl Display) -> String {
        format!("tg://resolve?domain=ChatWarsBot&text={command}")
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::*;
