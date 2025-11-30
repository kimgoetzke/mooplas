#![cfg(feature = "online")]

use bevy::log::*;
use bevy::prelude::{App, Plugin};
use bevy_renet::RenetServerPlugin;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(RenetServerPlugin);

    info!("Online multiplayer is enabled");
  }
}
