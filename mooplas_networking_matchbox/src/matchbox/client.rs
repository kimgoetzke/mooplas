use bevy::app::{App, Plugin, Update};
use bevy::log::*;
use bevy::prelude::{Commands, IntoScheduleConfigs, MessageReader, MessageWriter, ResMut, Resource, resource_exists};
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::{ChannelError, Packet, PeerId, PeerState};
use mooplas_networking::prelude::{
  ChannelType, ClientNetworkingActive, InboundServerMessage, NetworkErrorEvent, OutboundClientMessage,
  decode_from_bytes,
};

/// A Bevy plugin that adds client-side online multiplayer capabilities.
pub struct MatchboxClientPlugin;

impl Plugin for MatchboxClientPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        receive_server_messages_system.run_if(resource_exists::<ClientNetworkingActive>),
      )
      .add_systems(
        Update,
        handle_outbound_client_message.run_if(resource_exists::<ClientNetworkingActive>),
      );
  }
}

#[derive(Resource)]
pub struct HostConnectionInfo {
  pub host_id: PeerId,
}

fn network_error_from_peer_state(peer_state: PeerState) -> Option<NetworkErrorEvent> {
  match peer_state {
    PeerState::Connected => None,
    PeerState::Disconnected => Some(NetworkErrorEvent::Disconnect("Host disconnected".to_string())),
  }
}

fn network_error_from_channel_error(channel_error: ChannelError) -> NetworkErrorEvent {
  match channel_error {
    ChannelError::Closed => NetworkErrorEvent::Disconnect("Connection closed".to_string()),
    _ => NetworkErrorEvent::OtherError(channel_error.to_string()),
  }
}

fn receive_server_messages_system(
  mut socket: ResMut<MatchboxSocket>,
  mut commands: Commands,
  mut inbound_server_message: MessageWriter<InboundServerMessage>,
) {
  match socket.try_update_peers() {
    Ok(result) => {
      for (peer_id, state) in result {
        info!("[{peer_id}]: {state:?}");
        if let Some(error) = network_error_from_peer_state(state) {
          commands.trigger(error);
        }
      }
    }
    Err(channel_error) => {
      commands.trigger(network_error_from_channel_error(channel_error));
    }
  }

  for (_id, message) in socket.channel_mut(ChannelType::ReliableOrdered.into()).receive() {
    let server_message: InboundServerMessage =
      decode_from_bytes(&message).expect("Failed to deserialise server message");
    debug!(
      "Received [{:?}] server message: {:?}",
      ChannelType::ReliableOrdered,
      server_message
    );
    inbound_server_message.write(server_message);
  }

  for (_id, message) in socket.channel_mut(ChannelType::Unreliable.into()).receive() {
    let server_message: InboundServerMessage =
      decode_from_bytes(&message).expect("Failed to deserialise server message");
    inbound_server_message.write(server_message);
  }
}

/// A system that applies outgoing send/disconnect requests via [`MatchboxSocket`].
fn handle_outbound_client_message(
  mut messages: MessageReader<OutboundClientMessage>,
  mut socket: ResMut<MatchboxSocket>,
) {
  for message in messages.read() {
    match message {
      OutboundClientMessage::Send { channel, payload } => {
        let packet = Packet::from(payload.as_slice());
        let peers: Vec<_> = socket.connected_peers().collect();
        for peer_id in peers {
          socket.channel_mut((*channel).into()).send(packet.clone(), peer_id);
        }
      }
      OutboundClientMessage::Disconnect => {
        socket.close();
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn network_error_from_peer_state_returns_disconnect_when_host_peer_disconnects() {
    let error = network_error_from_peer_state(PeerState::Disconnected)
      .expect("Expected host peer disconnection to become network error");

    assert!(matches!(
      error,
      NetworkErrorEvent::Disconnect(message) if message == "Host disconnected"
    ));
  }

  #[test]
  fn network_error_from_peer_state_ignores_connected_host_peer() {
    assert!(network_error_from_peer_state(PeerState::Connected).is_none());
  }
}
