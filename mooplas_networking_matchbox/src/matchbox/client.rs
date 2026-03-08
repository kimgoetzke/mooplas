use bevy::app::{App, Plugin, Update};
use bevy::log::*;
use bevy::prelude::{Commands, IntoScheduleConfigs, MessageReader, ResMut, Resource, resource_exists};
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::{ChannelError, Packet, PeerId};
use mooplas_networking::prelude::{
  ChannelType, ClientNetworkingActive, NetworkErrorEvent, OutgoingClientMessage, ServerEvent, decode_from_bytes,
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
        send_outgoing_client_messages_system.run_if(resource_exists::<ClientNetworkingActive>),
      );
  }
}

#[derive(Resource)]
pub struct HostConnectionInfo {
  pub host_id: PeerId,
}

fn receive_server_messages_system(mut socket: ResMut<MatchboxSocket>, mut commands: Commands) {
  match socket.try_update_peers() {
    Ok(result) => {
      for (peer_id, state) in result {
        info!("[{peer_id}]: {state:?}");
      }
    }
    Err(channel_error) => {
      let error = match channel_error {
        ChannelError::Closed => NetworkErrorEvent::Disconnect("Connection closed".to_string()),
        _ => NetworkErrorEvent::OtherError(channel_error.to_string()),
      };

      commands.trigger(error);
    }
  }

  for (_id, message) in socket.channel_mut(ChannelType::ReliableOrdered.into()).receive() {
    let server_message: ServerEvent = decode_from_bytes(&message).expect("Failed to deserialise server message");
    debug!(
      "Received [{:?}] server message: {:?}",
      ChannelType::ReliableOrdered,
      server_message
    );
    commands.trigger(server_message);
  }

  for (_id, message) in socket.channel_mut(ChannelType::Unreliable.into()).receive() {
    let server_message: ServerEvent = decode_from_bytes(&message).expect("Failed to deserialise server message");
    commands.trigger(server_message);
  }
}

/// A system that applies outgoing send/disconnect requests via [`MatchboxSocket`].
fn send_outgoing_client_messages_system(
  mut outgoing_messages: MessageReader<OutgoingClientMessage>,
  mut socket: ResMut<MatchboxSocket>,
) {
  for outgoing_message in outgoing_messages.read() {
    match outgoing_message {
      OutgoingClientMessage::Send { channel, payload } => {
        let packet = Packet::from(payload.as_slice());
        let peers: Vec<_> = socket.connected_peers().collect();
        for peer_id in peers {
          socket.channel_mut((*channel).into()).send(packet.clone(), peer_id);
        }
      }
      OutgoingClientMessage::Disconnect => {
        socket.close();
      }
    }
  }
}
