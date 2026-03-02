#[cfg(feature = "online")]
use crate::online::networking::NetworkingPlugin;
use bevy::prelude::{App, Plugin};

mod utils;

#[cfg(feature = "online")]
mod structs;

#[cfg(feature = "online")]
mod networking;

#[cfg(feature = "online")]
mod shared_server;

#[cfg(feature = "online")]
mod shared_client;

#[cfg(feature = "online_renet")]
mod renet;

#[cfg(feature = "online_matchbox")]
mod matchbox;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  #[allow(unused_variables)]
  fn build(&self, app: &mut App) {
    #[cfg(feature = "online")]
    app.add_plugins(NetworkingPlugin);

    #[cfg(feature = "online_renet")]
    app.add_plugins(renet::RenetPlugin);

    #[cfg(feature = "online_matchbox")]
    app.add_plugins(matchbox::MatchboxPlugin);
  }
}
