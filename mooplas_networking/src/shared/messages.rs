use crate::prelude::NetworkErrorEvent;
use bevy::app::{App, Plugin};
use bevy::log::info;
use bevy::prelude::{Commands, Message, On};
use bevy_renet::netcode::{NetcodeError, NetcodeErrorEvent, NetcodeTransportError};
use serde::{Deserialize, Serialize};

pub struct NetworkingMessagesPlugin;

impl Plugin for NetworkingMessagesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<PlayerStateUpdateMessage>()
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

/// A message containing authoritative state updates for a player from the server. Used for server-to-client state
/// synchronisation.
#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlayerStateUpdateMessage {
  /// The [`PlayerId`] as a u8
  pub id: u8,
  /// Position (x, y) of the player's snake head
  pub position: (f32, f32),
  /// Rotation in radians around Z axis
  pub rotation: f32,
}

impl PlayerStateUpdateMessage {
  pub fn new(player_id: u8, position: (f32, f32), rotation: f32) -> Self {
    Self {
      id: player_id,
      position,
      rotation,
    }
  }
}
