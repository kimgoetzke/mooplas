#![cfg(feature = "online-multiplayer")]

use bevy::log::*;
use bevy::prelude::{App, Plugin};
use bevy_renet::RenetServerPlugin;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlineMultiplayerPlugin;

impl Plugin for OnlineMultiplayerPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(RenetServerPlugin);

    info!("Online multiplayer is enabled");
  }
}
