use crate::prelude::ClientNetworkingActive;
use bevy::app::{App, Plugin, Update};
use bevy::log::*;
use bevy::prelude::{IntoScheduleConfigs, ResMut, resource_exists};
use bevy::time::common_conditions::on_timer;
use bevy_matchbox::MatchboxSocket;
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

const CHANNEL_ID: usize = 0;

fn send_message(mut socket: ResMut<MatchboxSocket>) {
  let peers: Vec<_> = socket.connected_peers().collect();

  for peer in peers {
    let message = "Hello";
    info!("Sending message: {message:?} to {peer}");
    socket.channel_mut(CHANNEL_ID).send(message.as_bytes().into(), peer);
  }
}

fn receive_messages(mut socket: ResMut<MatchboxSocket>) {
  for (peer, state) in socket.update_peers() {
    info!("{peer}: {state:?}");
  }

  for (_id, message) in socket.channel_mut(CHANNEL_ID).receive() {
    match std::str::from_utf8(&message) {
      Ok(message) => info!("Received message: {message:?}"),
      Err(e) => error!("Failed to convert message to string: {e}"),
    }
  }
}
