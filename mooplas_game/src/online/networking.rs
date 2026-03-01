use crate::online::server::ServerPlugin;
use bevy::prelude::{App, Plugin};
use mooplas_networking::prelude::NetworkingResourcesPlugin;

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins((NetworkingResourcesPlugin, ServerPlugin));
  }
}
