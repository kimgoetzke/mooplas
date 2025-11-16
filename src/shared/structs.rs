use crate::prelude::PlayerId;
use bevy::prelude::KeyCode;

/// Represents a player that has registered to play the game. Used during the game loop.
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

/// Represents an available player input configuration. Predefined for players to choose from.
#[derive(Clone)]
pub struct AvailablePlayerInput {
  pub id: PlayerId,
  pub input: PlayerInput,
}
