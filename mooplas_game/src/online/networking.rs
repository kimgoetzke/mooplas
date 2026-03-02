use crate::online::shared_client::SharedClientPlugin;
use crate::online::shared_server::SharedServerPlugin;
use bevy::prelude::{App, Plugin};
use mooplas_networking::prelude::{NetworkingMessagesPlugin, NetworkingResourcesPlugin};

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins((
      NetworkingResourcesPlugin,
      NetworkingMessagesPlugin,
      SharedServerPlugin,
      SharedClientPlugin,
    ));
  }
}
