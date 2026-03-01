use crate::prelude::client_id_from_peer_id;
use bevy::prelude::*;
use bevy_matchbox::matchbox_socket::Packet;
use bevy_matchbox::{matchbox_signaling::SignalingServer, prelude::*};
use crossbeam_channel::Receiver;
use mooplas_networking::prelude::{
  ChannelType, ClientMessage, Lobby, OutgoingServerMessage, ServerEvent, ServerNetworkingActive, decode_from_bytes,
};
use std::net::{Ipv4Addr, SocketAddrV4};

/// Resource to receive client events from callbacks
#[derive(Resource)]
pub struct ClientConnectionReceiver(pub Receiver<ServerEvent>);

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
        send_outgoing_server_messages_system.run_if(resource_exists::<ServerNetworkingActive>),
      );
  }
}

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
      .on_client_connected(move |id| trace!("Client connected with ID [{id}]"))
      .on_client_disconnected(|id| trace!("Client [{id}] left"))
      .cors()
      .trace()
      .build(),
  );

  commands.insert_resource(matchbox_server);
}

fn receive_messages(mut socket: ResMut<MatchboxSocket>, mut commands: Commands, mut lobby: ResMut<Lobby>) {
  for (peer_id, state) in socket.update_peers() {
    info!("[{peer_id}]: {state:?}");
    let client_id = client_id_from_peer_id(peer_id);
    let server_event: ServerEvent = match state {
      PeerState::Connected => {
        trace!("Client with ID [{client_id}] connected");
        lobby.connected.push(client_id);
        ServerEvent::ClientConnected { client_id }
      }
      PeerState::Disconnected => {
        trace!("Client with ID [{client_id}] disconnected");
        lobby.connected.retain(|&id| id != client_id);
        ServerEvent::ClientDisconnected { client_id }
      }
    };

    // Trigger an event for an application to react to
    commands.trigger(server_event);
  }

  for (peer_id, message) in socket.channel_mut(ChannelType::ReliableOrdered.into()).receive() {
    let client_id = client_id_from_peer_id(peer_id);
    let client_message: ClientMessage = decode_from_bytes(&message).expect("Failed to deserialise client message");
    trace!(
      "Received [{:?}] message from client [{client_id}]: {:?}",
      ChannelType::ReliableOrdered,
      client_message
    );
    commands.trigger(client_message.to_event(client_id));
  }

  for (peer_id, message) in socket.channel_mut(ChannelType::Unreliable.into()).receive() {
    let client_id = client_id_from_peer_id(peer_id);
    let client_message: ClientMessage = decode_from_bytes(&message).expect("Failed to deserialise client message");
    trace!(
      "Received [{:?}] message from client [{client_id}]: {:?}",
      ChannelType::Unreliable,
      client_message
    );
    commands.trigger(client_message.to_event(client_id));
  }
}

/// A system that applies outgoing send/broadcast/disconnect requests using the [`MatchboxSocket`].
fn send_outgoing_server_messages_system(
  mut outgoing_messages: MessageReader<OutgoingServerMessage>,
  mut socket: ResMut<MatchboxSocket>,
) {
  for outgoing_message in outgoing_messages.read() {
    match outgoing_message {
      OutgoingServerMessage::Broadcast { channel, payload } => {
        let packet = Packet::from(payload.as_slice());
        let peers: Vec<PeerId> = socket.connected_peers().collect();
        for peer_id in peers {
          socket.channel_mut((*channel).into()).send(packet.clone(), peer_id);
        }
      }
      OutgoingServerMessage::BroadcastExcept {
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
      OutgoingServerMessage::Send {
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
      OutgoingServerMessage::DisconnectAll => {
        socket.close();
      }
    }
  }
}
