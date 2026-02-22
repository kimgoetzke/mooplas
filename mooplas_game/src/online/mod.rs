use bevy::prelude::{App, Plugin};
use mooplas_networking::prelude::NetworkingResourcesPlugin;

mod utils;

#[cfg(feature = "online_renet")]
mod native;

#[cfg(feature = "online_matchbox")]
mod wasm;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(NetworkingResourcesPlugin);

    #[cfg(feature = "online_renet")]
    app.add_plugins(native::NativeOnlinePlugin);

    #[cfg(feature = "online_matchbox")]
    app.add_plugins(wasm::WasmOnlinePlugin);
  }
}
