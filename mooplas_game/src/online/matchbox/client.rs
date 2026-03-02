use bevy::prelude::{App, Plugin};
use mooplas_networking_matchbox::prelude::MatchboxClientPlugin;

/// A plugin that adds client-side online multiplayer capabilities to the game. Only active when the application is
/// running in client mode (i.e. someone else is the server). Mutually exclusive with the
/// [`crate::online::renet::ClientPlugin`] but must be used in addition to
/// [`crate::online::shared_client::SharedClientPlugin`].
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
  fn build(&self, app: &mut App) {
    app.add_plugins(MatchboxClientPlugin);
  }
}
