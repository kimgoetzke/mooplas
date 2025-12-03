use crate::prelude::{NetworkRole, PlayerId};
use bevy::prelude::{Color, KeyCode};
use serde::{Deserialize, Serialize};

/// Represents a player that has registered to play the game. Used during the game loop.
#[derive(Clone)]
pub struct RegisteredPlayer {
  pub id: PlayerId,
  pub input: PlayerInput,
  pub colour: Color,
  pub alive: bool,
  pub mutable: bool,
}

impl From<&AvailablePlayerConfig> for RegisteredPlayer {
  fn from(config: &AvailablePlayerConfig) -> Self {
    Self {
      id: config.id,
      input: config.input.clone(),
      colour: config.colour,
      alive: true,
      mutable: true,
    }
  }
}

impl RegisteredPlayer {
  pub fn new_mutable(id: PlayerId, input: PlayerInput, colour: Color) -> Self {
    Self {
      id,
      input,
      colour,
      alive: true,
      mutable: true,
    }
  }

  pub fn new_mutable_from(config: &AvailablePlayerConfig) -> Self {
    Self {
      id: config.id,
      input: config.input.clone(),
      colour: config.colour,
      alive: true,
      mutable: true,
    }
  }

  pub fn new_immutable(id: PlayerId, input: PlayerInput, colour: Color) -> Self {
    Self {
      id,
      input,
      colour,
      alive: true,
      mutable: false,
    }
  }

  pub fn new_immutable_from(config: &AvailablePlayerConfig) -> Self {
    Self {
      id: config.id,
      input: config.input.clone(),
      colour: config.colour,
      alive: true,
      mutable: false,
    }
  }
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum NetworkAudience {
  Server,
  Client,
}

impl From<NetworkRole> for Option<NetworkAudience> {
  fn from(role: NetworkRole) -> Self {
    match role {
      NetworkRole::Server => Some(NetworkAudience::Server),
      NetworkRole::Client => Some(NetworkAudience::Client),
      NetworkRole::None => None,
    }
  }
}

impl NetworkAudience {
  pub fn is_server(&self) -> bool {
    matches!(self, NetworkAudience::Server)
  }

  pub fn is_client(&self) -> bool {
    matches!(self, NetworkAudience::Client)
  }
}
