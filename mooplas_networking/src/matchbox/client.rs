use crate::prelude::ClientNetworkingActive;
use bevy::app::{App, Plugin, Update};
use bevy::log::*;
use bevy::prelude::{IntoScheduleConfigs, ResMut, Resource, resource_exists};
use bevy::time::common_conditions::on_timer;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::PeerId;
use std::time::Duration;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        receive_messages.run_if(resource_exists::<ClientNetworkingActive>),
      )
      .add_systems(
        Update,
        send_message
          .run_if(resource_exists::<ClientNetworkingActive>)
          .run_if(on_timer(Duration::from_secs(5))),
      );
  }
}

#[derive(Resource)]
pub struct HostConnectionInfo {
  pub host_id: PeerId,
}

const CHANNEL_ID: usize = 0;

fn send_message(mut socket: ResMut<MatchboxSocket>) {
  let peers: Vec<_> = socket.connected_peers().collect();
  for peer_id in peers {
    let message = format!("Hello, I'm client [{peer_id}]");
    info!("Sending message: {message:?} to {peer_id}");
    socket.channel_mut(CHANNEL_ID).send(message.as_bytes().into(), peer_id);
  }
}

fn receive_messages(mut socket: ResMut<MatchboxSocket>) {
  for (peer_id, state) in socket.update_peers() {
    info!("[{peer_id}]: {state:?}");
  }

  for (_id, message) in socket.channel_mut(CHANNEL_ID).receive() {
    match std::str::from_utf8(&message) {
      Ok(message) => info!("Received message: {message:?}"),
      Err(e) => error!("Failed to convert message to string: {e}"),
    }
  }
}
