use crate::online::client::ClientPlugin;
use crate::online::server::ServerPlugin;
use bevy::prelude::{App, Plugin};
use mooplas_networking::prelude::{NetworkingMessagesPlugin, NetworkingResourcesPlugin};

/// Plugin that adds online multiplayer capabilities that are shared across all online multiplayer implementations.
pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins((
      NetworkingResourcesPlugin,
      NetworkingMessagesPlugin,
      ServerPlugin,
      ClientPlugin,
    ));
  }
}
