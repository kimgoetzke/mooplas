use bevy::app::{App, Plugin};
use bevy::log::info;
use bevy::prelude::{Commands, Message, On};
use bevy_renet::netcode::{NetcodeError, NetcodeErrorEvent, NetcodeTransportError};
use mooplas_networking::prelude::NetworkErrorEvent;

/// A plugin that adds messages related to the Renet networking implementation.
pub struct RenetNetworkingMessagesPlugin;

impl Plugin for RenetNetworkingMessagesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<ClientHandshakeOutcomeMessage>()
      .add_observer(receive_netcode_transport_error_event);
  }
}

/// A message sent on the client-side by the networking code after the client handshake process has completed. Contains
/// the result of the handshake. Can optionally be used by the application client-side code to e.g. trigger UI error
/// messages in case of failures.
#[derive(Message, Debug)]
pub struct ClientHandshakeOutcomeMessage {
  pub has_succeeded: bool,
  pub reason: Option<String>,
}

/// An observer that listens for errors emitted by the Renet transport and triggers a more generic
/// [`NetworkErrorEvent`] that can be handled by the application code.
#[allow(clippy::never_loop)]
fn receive_netcode_transport_error_event(error_event: On<NetcodeErrorEvent>, mut commands: Commands) {
  let netcode_transport_error = &(**error_event);
  info!("Netcode transport error occurred: [{}]...", netcode_transport_error);
  let error = map_netcode_transport_error_to_network_error(netcode_transport_error);
  commands.trigger(error);
}

fn map_netcode_transport_error_to_network_error(netcode_transport_error: &NetcodeTransportError) -> NetworkErrorEvent {
  match netcode_transport_error {
    NetcodeTransportError::Renet(e) => NetworkErrorEvent::RenetDisconnect(e.to_string()),
    NetcodeTransportError::Netcode(e) => match e {
      NetcodeError::Disconnected(reason) => NetworkErrorEvent::NetcodeDisconnect(reason.to_string()),
      _ => NetworkErrorEvent::NetcodeTransportError(e.to_string()),
    },
    NetcodeTransportError::IO(e) => NetworkErrorEvent::IoError(e.to_string()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use bevy_renet::netcode::{NetcodeError, NetcodeTransportError};
  use mooplas_networking::prelude::NetworkErrorEvent;

  #[test]
  fn map_netcode_transport_error_to_network_error_maps_netcode_disconnected_to_netcode_disconnect_event() {
    use bevy_renet::netcode::NetcodeDisconnectReason;
    let net_err = NetcodeTransportError::Netcode(NetcodeError::Disconnected(
      NetcodeDisconnectReason::DisconnectedByServer,
    ));
    let res = map_netcode_transport_error_to_network_error(&net_err);
    match res {
      NetworkErrorEvent::NetcodeDisconnect(s) => assert!(s.contains("server") || s.contains("terminated")),
      _ => panic!("Expected NetworkErrorEvent::NetcodeDisconnect"),
    }
  }

  #[test]
  fn map_netcode_transport_error_to_network_error_maps_io_error_to_io_error_event() {
    let io_err = NetcodeTransportError::IO(std::io::Error::new(std::io::ErrorKind::Other, "io"));
    let res = map_netcode_transport_error_to_network_error(&io_err);
    match res {
      NetworkErrorEvent::IoError(s) => assert!(s.contains("io")),
      _ => panic!("Expected NetworkErrorEvent::IoError"),
    }
  }
}
