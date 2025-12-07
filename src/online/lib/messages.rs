use crate::prelude::{InputAction, PlayerId};
use avian2d::math::Scalar;
use bevy::app::{App, Plugin};
use bevy::prelude::Message;
use serde::{Deserialize, Serialize};

pub struct NetworkingMessagesPlugin;

impl Plugin for NetworkingMessagesPlugin {
  fn build(&self, app: &mut App) {
    app.add_message::<SerialisableInputAction>();
  }
}

#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SerialisableInputAction {
  Move(u8, Scalar),
  Action(u8),
}

impl Default for SerialisableInputAction {
  fn default() -> Self {
    SerialisableInputAction::Move(0, 0.0)
  }
}

impl From<&InputAction> for SerialisableInputAction {
  fn from(value: &InputAction) -> Self {
    match value {
      InputAction::Move(player_id, direction) => SerialisableInputAction::Move(player_id.0, *direction),
      InputAction::Action(player_id) => SerialisableInputAction::Action(player_id.0),
    }
  }
}

impl Into<InputAction> for SerialisableInputAction {
  fn into(self) -> InputAction {
    match self {
      SerialisableInputAction::Move(player_id, direction) => InputAction::Move(PlayerId(player_id), direction),
      SerialisableInputAction::Action(player_id) => InputAction::Action(PlayerId(player_id)),
    }
  }
}
