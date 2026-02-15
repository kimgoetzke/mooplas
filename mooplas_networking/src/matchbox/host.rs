use crate::shared::ServerNetworkingActive;
use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_matchbox::{matchbox_signaling::SignalingServer, prelude::*};
use core::time::Duration;
use std::net::{Ipv4Addr, SocketAddrV4};

pub struct HostPlugin;

impl Plugin for HostPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        receive_messages.run_if(resource_exists::<ServerNetworkingActive>),
      )
      .add_systems(
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
  let signaling_server = MatchboxServer::from(
    SignalingServer::client_server_builder(addr)
      .on_connection_request(|connection| {
        info!("Connecting: {connection:?}");
        Ok(true)
      })
      .on_id_assignment(|(socket, id)| info!("{socket} received {id}"))
      .on_host_connected(|id| info!("Host joined: {id}"))
      .on_host_disconnected(|id| info!("Host left: {id}"))
      .on_client_connected(|id| info!("Client joined: {id}"))
      .on_client_disconnected(|id| info!("Client left: {id}"))
      .cors()
      .trace()
      .build(),
  );
  commands.insert_resource(signaling_server);
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
  for (peer, state) in socket.update_peers() {
    info!("{peer}: {state:?}");
  }

  for (_id, message) in socket.channel_mut(0).receive() {
    match std::str::from_utf8(&message) {
      Ok(message) => info!("Received message: {message:?}"),
      Err(e) => error!("Failed to convert message to string: {e}"),
    }
  }
}
