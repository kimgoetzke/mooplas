use bevy::prelude::{App, Plugin};

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  #[allow(unused_variables)]
  fn build(&self, app: &mut App) {
    #[cfg(feature = "online")]
    app.add_plugins((
      mooplas_networking::prelude::NetworkingResourcesPlugin,
      mooplas_networking::prelude::NetworkingMessagesPlugin,
      crate::online::server::ServerPlugin,
      crate::online::client::ClientPlugin,
    ));

    #[cfg(feature = "online_renet")]
    app.add_plugins(crate::online::renet::RenetPlugin);

    #[cfg(feature = "online_matchbox")]
    app.add_plugins(crate::online::matchbox::MatchboxPlugin);
  }
}
