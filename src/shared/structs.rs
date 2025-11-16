use crate::prelude::PlayerId;
use bevy::prelude::KeyCode;

#[derive(Clone)]
pub struct RegisteredPlayer {
  pub id: PlayerId,
  pub input: PlayerInput,
  pub alive: bool,
}

/// Defines the key bindings for a given player.
#[derive(Clone)]
pub struct PlayerInput {
  pub id: PlayerId,
  pub left: KeyCode,
  pub right: KeyCode,
  pub action: KeyCode,
}

impl PlayerInput {
  pub fn new(id: PlayerId, left: KeyCode, right: KeyCode, action: KeyCode) -> Self {
    Self {
      id,
      left,
      right,
      action,
    }
  }
}
