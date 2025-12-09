use crate::prelude::{InputMessage, PlayerId};
use avian2d::math::Scalar;
use bevy::app::{App, Plugin};
use bevy::prelude::Message;
use serde::{Deserialize, Serialize};

pub struct NetworkingMessagesPlugin;

impl Plugin for NetworkingMessagesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<SerialisableInputActionMessage>()
      .add_message::<PlayerStateUpdateMessage>();
  }
}

#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SerialisableInputActionMessage {
  Move(u8, Scalar),
  Action(u8),
}

impl Default for SerialisableInputActionMessage {
  fn default() -> Self {
    SerialisableInputActionMessage::Move(0, 0.0)
  }
}

impl From<&InputMessage> for SerialisableInputActionMessage {
  fn from(value: &InputMessage) -> Self {
    match value {
      InputMessage::Move(player_id, direction) => SerialisableInputActionMessage::Move(player_id.0, *direction),
      InputMessage::Action(player_id) => SerialisableInputActionMessage::Action(player_id.0),
    }
  }
}

impl Into<InputMessage> for SerialisableInputActionMessage {
  fn into(self) -> InputMessage {
    match self {
      SerialisableInputActionMessage::Move(player_id, direction) => InputMessage::Move(PlayerId(player_id), direction),
      SerialisableInputActionMessage::Action(player_id) => InputMessage::Action(PlayerId(player_id)),
    }
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
