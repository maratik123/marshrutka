macro_rules! deep_link {
    ($mod_name:ident, $link_to_support_chat:literal, $send_command_to_bot:literal, $send_command:literal) => {
        mod $mod_name {
            use std::fmt::Display;

            pub const LINK_TO_SUPPORT_CHAT: &str = $link_to_support_chat;

            pub fn send_command_to_bot(command: impl Display) -> String {
                format!($send_command_to_bot, command)
            }

            pub fn send_command(command: impl Display) -> String {
                format!($send_command, command)
            }
        }
    };
}

#[cfg(not(target_arch = "wasm32"))]
deep_link!(
    native,
    "https://t.me/marshrutka_support",
    "https://t.me/ChatWarsBot?text={}",
    "https://t.me/share?url={}"
);

#[cfg(target_arch = "wasm32")]
deep_link!(
    wasm,
    "tg://resolve?domain=marshrutka_support",
    "tg://resolve?domain=ChatWarsBot&text={}",
    "tg://msg_url?url={}"
);

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
