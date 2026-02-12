use bevy::prelude::{App, Plugin};

mod native;
mod utils;
mod wasm;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(native::NativeOnlinePlugin);

    #[cfg(target_arch = "wasm32")]
    app.add_plugins(wasm::WasmOnlinePlugin);
  }
}
