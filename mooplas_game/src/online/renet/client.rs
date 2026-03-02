use crate::prelude::{AppState, UiNotification};
use bevy::prelude::{App, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Update, in_state};
use mooplas_networking_renet::prelude::{ClientHandshakeOutcomeMessage, ClientRenetPlugin, ClientVisualiserPlugin};

/// A plugin that adds client-side online multiplayer capabilities to the game. Only active when the application is
/// running in client mode (i.e. someone else is the server). Mutually exclusive with the
/// [`crate::online::matchbox::ServerPlugin`] but must be used in addition to
/// [`crate::online::shared_client::SharedClientPlugin`].
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((ClientRenetPlugin, ClientVisualiserPlugin))
      .add_systems(Update, client_handshake_system.run_if(in_state(AppState::Preparing)));
  }
}

/// System that checks whether the client completed the handshake before the deadline.
/// If the handshake did not complete in time, it cleans up the client transport and
/// emits a UI error message.
pub fn client_handshake_system(
  mut messages: MessageReader<ClientHandshakeOutcomeMessage>,
  mut ui_message: MessageWriter<UiNotification>,
) {
  for message in messages.read() {
    let reason = message
      .reason
      .as_ref()
      .expect("Handshake outcome message should always contain a reason");
    match message.has_succeeded {
      true => ui_message.write(UiNotification::info(reason.to_string())),
      false => ui_message.write(UiNotification::error(reason.to_string())),
    };
  }
}
