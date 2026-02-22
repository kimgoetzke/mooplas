use bevy::app::{App, Plugin};
use bevy::log::info;
use bevy::prelude::{Commands, Message, On};
use bevy_renet::netcode::{NetcodeError, NetcodeErrorEvent, NetcodeTransportError};
use mooplas_networking::prelude::NetworkErrorEvent;

pub struct RenetNetworkingMessagesPlugin;

impl Plugin for RenetNetworkingMessagesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<ClientHandshakeOutcomeMessage>()
      .add_observer(receive_netcode_transport_error_event);
  }
}

#[allow(clippy::never_loop)]
fn receive_netcode_transport_error_event(error_event: On<NetcodeErrorEvent>, mut commands: Commands) {
  let netcode_transport_error = &(**error_event);
  info!("Netcode transport error occurred: [{}]...", netcode_transport_error);
  let error = match netcode_transport_error {
    NetcodeTransportError::Renet(e) => NetworkErrorEvent::RenetDisconnect(e.to_string()),
    NetcodeTransportError::Netcode(e) => match e {
      NetcodeError::Disconnected(reason) => NetworkErrorEvent::NetcodeDisconnect(reason.to_string()),
      _ => NetworkErrorEvent::NetcodeTransportError(e.to_string()),
    },
    NetcodeTransportError::IO(e) => NetworkErrorEvent::IoError(e.to_string()),
  };
  commands.trigger(error);
}

/// A message sent on the client-side by the networking code after the client handshake process has completed. Contains
/// the result of the handshake. Can optionally be used by the application client-side code to e.g. trigger UI error
/// messages in case of failures.
#[derive(Message, Debug)]
pub struct ClientHandshakeOutcomeMessage {
  pub has_succeeded: bool,
  pub reason: Option<String>,
}
