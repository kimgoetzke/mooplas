use bevy::prelude::{App, Plugin};
use mooplas_networking::prelude::NetworkingResourcesPlugin;

mod native;
mod utils;
mod wasm;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(NetworkingResourcesPlugin);

    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(native::NativeOnlinePlugin);

    #[cfg(target_arch = "wasm32")]
    app.add_plugins(wasm::WasmOnlinePlugin);
  }
}
