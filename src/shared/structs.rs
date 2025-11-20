use crate::prelude::PlayerId;
use bevy::prelude::{Color, KeyCode};

/// Represents a player that has registered to play the game. Used during the game loop.
#[derive(Clone)]
pub struct RegisteredPlayer {
  pub id: PlayerId,
  pub input: PlayerInput,
  pub colour: Color,
  pub alive: bool,
}

/// Defines the key bindings for a given player.
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct AvailablePlayerConfig {
  pub id: PlayerId,
  pub input: PlayerInput,
  pub colour: Color,
}

impl AvailablePlayerConfig {
  pub fn id(&self) -> PlayerId {
    self.id
  }
}

impl Into<PlayerId> for &AvailablePlayerConfig {
  fn into(self) -> PlayerId {
    self.id
  }
}
