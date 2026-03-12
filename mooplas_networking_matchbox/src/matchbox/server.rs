use crate::prelude::client_id_from_peer_id;
use bevy::prelude::*;
use bevy_matchbox::matchbox_socket::{ChannelError, Packet};
use bevy_matchbox::{matchbox_signaling::SignalingServer, prelude::*};
use mooplas_networking::prelude::{
  ChannelType, ClientMessage, InboundClientMessage, InboundServerMessage, Lobby, NetworkErrorEvent,
  OutboundServerMessage, ServerNetworkingActive, decode_from_bytes,
};
use std::net::{Ipv4Addr, SocketAddrV4};

/// A Bevy plugin that adds server-side online multiplayer capabilities using Matchbox.
pub struct ServerMatchboxPlugin;

impl Plugin for ServerMatchboxPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        receive_messages.run_if(resource_exists::<ServerNetworkingActive>),
      )
      .add_systems(
        Update,
        handle_outbound_server_message.run_if(resource_exists::<ServerNetworkingActive>),
      );
  }
}

/// A system that starts the Matchbox signaling server and inserts the [`MatchboxServer`] resource.
pub fn start_signaling_server(commands: &mut Commands) {
  info!("Starting signaling server");
  let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 3536);
  let matchbox_server = MatchboxServer::from(
    SignalingServer::client_server_builder(addr)
      .on_connection_request(|connection| {
        info!("Connecting: {connection:?}");
        Ok(true)
      })
      .on_id_assignment(|(socket, id)| info!("Socket [{socket}] received ID [{id}]"))
      .on_host_connected(|id| info!("Host joined and has ID [{id}]"))
      .on_host_disconnected(|id| info!("Host [{id}] left"))
      .on_client_connected(move |id| trace!("Client with ID [{id}] connected"))
      .on_client_disconnected(|id| trace!("Client [{id}] left"))
      .cors()
      .trace()
      .build(),
  );

  commands.insert_resource(matchbox_server);
}

/// A system that receives incoming messages and connection events from the [`MatchboxSocket`] and triggers
/// corresponding events for the application to react to. Also updates the [`Lobby`] resource with connected clients.
fn receive_messages(
  mut socket: ResMut<MatchboxSocket>,
  mut commands: Commands,
  mut lobby: ResMut<Lobby>,
  mut inbound_client_message: MessageWriter<InboundClientMessage>,
  mut inbound_server_message: MessageWriter<InboundServerMessage>,
) {
  match socket.try_update_peers() {
    Ok(result) => {
      for (peer_id, state) in result {
        let client_id = client_id_from_peer_id(peer_id);
        let server_event: InboundServerMessage = match state {
          PeerState::Connected => {
            trace!("Client with ID [{client_id}] connected");
            lobby.connected.push(client_id);
            InboundServerMessage::ClientConnected { client_id }
          }
          PeerState::Disconnected => {
            trace!("Client with ID [{client_id}] disconnected");
            lobby.connected.retain(|&id| id != client_id);
            InboundServerMessage::ClientDisconnected { client_id }
          }
        };

        // Trigger an event for an application to react to
        inbound_server_message.write(server_event);
      }
    }
    Err(channel_error) => {
      let error = match channel_error {
        ChannelError::Closed => NetworkErrorEvent::Disconnect("Connection closed".to_string()),
        _ => NetworkErrorEvent::OtherError(channel_error.to_string()),
      };

      // Trigger an error event for an application to react to
      commands.trigger(error);
    }
  }

  for (peer_id, message) in socket.channel_mut(ChannelType::ReliableOrdered.into()).receive() {
    let client_id = client_id_from_peer_id(peer_id);
    let client_message: ClientMessage = decode_from_bytes(&message).expect("Failed to deserialise client message");
    trace!(
      "Received [{:?}] message from client [{client_id}]: {:?}",
      ChannelType::ReliableOrdered,
      client_message
    );
    inbound_client_message.write(client_message.to_inbound_message(client_id));
  }

  for (peer_id, message) in socket.channel_mut(ChannelType::Unreliable.into()).receive() {
    let client_id = client_id_from_peer_id(peer_id);
    let client_message: ClientMessage = decode_from_bytes(&message).expect("Failed to deserialise client message");
    inbound_client_message.write(client_message.to_inbound_message(client_id));
  }
}

/// A system that applies outgoing send/broadcast/disconnect requests using the [`MatchboxSocket`].
fn handle_outbound_server_message(
  mut messages: MessageReader<OutboundServerMessage>,
  mut socket: ResMut<MatchboxSocket>,
) {
  for message in messages.read() {
    match message {
      OutboundServerMessage::Broadcast { channel, payload } => {
        let packet = Packet::from(payload.as_slice());
        let peers: Vec<PeerId> = socket.connected_peers().collect();
        for peer_id in peers {
          socket.channel_mut((*channel).into()).send(packet.clone(), peer_id);
        }
      }
      OutboundServerMessage::BroadcastExcept {
        except_client_id,
        channel,
        payload,
      } => {
        let packet = Packet::from(payload.as_slice());
        let peers: Vec<PeerId> = socket
          .connected_peers()
          .filter(|&peer_id| client_id_from_peer_id(peer_id) != *except_client_id)
          .collect();
        for peer_id in peers {
          socket.channel_mut((*channel).into()).send(packet.clone(), peer_id);
        }
      }
      OutboundServerMessage::Send {
        client_id,
        channel,
        payload,
      } => {
        let packet = Packet::from(payload.as_slice());
        let peer: PeerId = socket
          .connected_peers()
          .find(|&peer_id| client_id_from_peer_id(peer_id) == *client_id)
          .expect("Client ID not found among connected peers");
        socket.channel_mut((*channel).into()).send(packet, peer);
      }
      OutboundServerMessage::DisconnectAll => {
        socket.close();
      }
    }
  }
}
