use crate::matchbox::client_id_from_peer_id;
use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_matchbox::{matchbox_signaling::SignalingServer, prelude::*};
use core::time::Duration;
use crossbeam_channel::{Receiver, unbounded};
use mooplas_networking::prelude::{Lobby, ServerNetworkingActive};
use std::net::{Ipv4Addr, SocketAddrV4};

/// Resource to receive client connection events from callbacks
#[derive(Resource)]
pub struct ClientConnectionReceiver(pub Receiver<PeerId>);

pub struct HostPlugin;

impl Plugin for HostPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(
      Update,
      (receive_messages, handle_client_connection_system).run_if(resource_exists::<ServerNetworkingActive>),
    );
    app.add_systems(
      Update,
      send_message
        .run_if(resource_exists::<ServerNetworkingActive>)
        .run_if(on_timer(Duration::from_secs(5))),
    );
  }
}

pub fn start_signaling_server(commands: &mut Commands) {
  info!("Starting signaling server");
  let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 3536);
  let (sender, receiver) = unbounded::<PeerId>();
  let signaling_server = MatchboxServer::from(
    SignalingServer::client_server_builder(addr)
      .on_connection_request(|connection| {
        info!("Connecting: {connection:?}");
        Ok(true)
      })
      .on_id_assignment(|(socket, id)| info!("Socket [{socket}] received ID [{id}]"))
      .on_host_connected(|id| info!("Host joined and has ID [{id}]"))
      .on_host_disconnected(|id| info!("Host [{id}] left"))
      .on_client_connected(move |id| {
        info!("Client connected with ID [{id}]");
        if let Err(e) = sender.send(id) {
          error!("Failed to send client connection event: {e}");
        }
      })
      .on_client_disconnected(|id| info!("Client [{id}] left"))
      .cors()
      .trace()
      .build(),
  );
  commands.insert_resource(signaling_server);
  commands.insert_resource(ClientConnectionReceiver(receiver));
}

fn handle_client_connection_system(receiver: Res<ClientConnectionReceiver>, mut lobby: ResMut<Lobby>) {
  while let Ok(peer_id) = receiver.0.try_recv() {
    info!("Processing client connection for peer [{peer_id}] in Bevy system");
    lobby.connected.push(client_id_from_peer_id(peer_id));
  }
}

fn send_message(mut socket: ResMut<MatchboxSocket>) {
  let peers: Vec<_> = socket.connected_peers().collect();
  for peer in peers {
    let message = "Hello, I'm the host";
    info!("Sending message: {message:?} to {peer}");
    socket.channel_mut(0).send(message.as_bytes().into(), peer);
  }
}

fn receive_messages(mut socket: ResMut<MatchboxSocket>) {
  for (peer_id, state) in socket.update_peers() {
    info!("[{peer_id}]: {state:?}");
  }

  for (_id, message) in socket.channel_mut(0).receive() {
    match std::str::from_utf8(&message) {
      Ok(message) => info!("Received message: {message:?}"),
      Err(e) => error!("Failed to convert message to string: {e}"),
    }
  }
}
