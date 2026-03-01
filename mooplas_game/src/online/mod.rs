use crate::online::networking::NetworkingPlugin;
use bevy::prelude::{App, Plugin};

mod utils;

#[cfg(feature = "online")]
mod structs;

#[cfg(feature = "online")]
mod networking;

#[cfg(feature = "online")]
mod server;

#[cfg(feature = "online_renet")]
mod native;

#[cfg(feature = "online_matchbox")]
mod wasm;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    #[cfg(feature = "online")]
    app.add_plugins(NetworkingPlugin);

    #[cfg(feature = "online_renet")]
    app.add_plugins(native::NativeOnlinePlugin);

    #[cfg(feature = "online_matchbox")]
    app.add_plugins(wasm::WasmOnlinePlugin);
  }
}
