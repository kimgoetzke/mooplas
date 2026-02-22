use crate::prelude::{ChannelType, ClientId};
use bevy::app::{App, Plugin};
use bevy::prelude::Message;
use serde::{Deserialize, Serialize};

pub struct NetworkingMessagesPlugin;

impl Plugin for NetworkingMessagesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<PlayerStateUpdateMessage>()
      .add_message::<OutgoingClientMessage>()
      .add_message::<OutgoingServerMessage>();
  }
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

/// A request for the active client transport to send a payload to the server. This is intentionally transport-agnostic.
/// Should be used by application client-side code.
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
pub enum OutgoingClientMessage {
  Send { channel: ChannelType, payload: Vec<u8> },
  Disconnect,
}

/// A request for the active server transport to send/broadcast a payload to clients. This is intentionally
/// transport-agnostic. Should be used by application server-side code.
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
pub enum OutgoingServerMessage {
  /// Broadcast to all connected clients.
  Broadcast { channel: ChannelType, payload: Vec<u8> },
  /// Broadcast to all connected clients except the provided client.
  BroadcastExcept {
    except_client_id: ClientId,
    channel: ChannelType,
    payload: Vec<u8>,
  },
  /// Send to a specific client.
  Send {
    client_id: ClientId,
    channel: ChannelType,
    payload: Vec<u8>,
  },
  /// Disconnect all connected clients.
  DisconnectAll,
}
