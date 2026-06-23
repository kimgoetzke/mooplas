use crate::ui::shared::ButtonAnimation;

#[cfg(feature = "online")]
mod join_game_menu;

#[cfg(feature = "online")]
mod enter_name_menu;

#[cfg(feature = "online")]
mod host_game_menu;

mod in_game_ui;
mod main_menu;
mod notification;
mod play_online_menu;
mod shared;
mod touch_controls_ui;
mod ui;

pub use ui::UiPlugin;
