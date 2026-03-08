use bevy::prelude::{App, Plugin};

#[cfg(feature = "online")]
use crate::online::client::ClientPlugin;

#[cfg(feature = "online")]
use crate::online::server::ServerPlugin;

#[cfg(feature = "online_renet")]
use crate::online::renet;

#[cfg(feature = "online_matchbox")]
use crate::online::matchbox;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  #[allow(unused_variables)]
  fn build(&self, app: &mut App) {
    #[cfg(feature = "online")]
    app.add_plugins((
      mooplas_networking::prelude::NetworkingResourcesPlugin,
      mooplas_networking::prelude::NetworkingMessagesPlugin,
      ServerPlugin,
      ClientPlugin,
    ));

    #[cfg(feature = "online_renet")]
    app.add_plugins(renet::RenetPlugin);

    #[cfg(feature = "online_matchbox")]
    app.add_plugins(matchbox::MatchboxPlugin);
  }
}
