use bevy::app::{App, Plugin};
use mooplas_networking_renet::prelude::{ServerRenetPlugin, ServerVisualiserPlugin};

/// A plugin that adds server-side online multiplayer capabilities to the game. Only active when the game is running in
/// server mode. Mutually exclusive with the [`crate::online::matchbox::MatchboxPlugin`] but must be used in addition to
/// [`crate::online::shared_server::SharedServerPlugin`].
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins((ServerRenetPlugin, ServerVisualiserPlugin));
  }
}
