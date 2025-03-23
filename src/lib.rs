use rust_i18n::i18n;

i18n!("locales", fallback = "en");

pub mod app;
mod binary_heap;
mod cell;
mod consts;
mod cost;
mod deep_link;
mod emoji;
mod grid;
mod homeland;
mod index;
mod pathfinder;
mod skill;
mod translation;
