use crate::prelude::PlayerId;
use bevy::prelude::{Color, KeyCode};

pub const MAX_PLAYERS: u8 = 8;

/// A local-only identifier for a control scheme. Separates "which keys you're pressing" (local)
/// from "which player you are" (global/network).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControlSchemeId(pub u8);

/// Defines a set of key bindings that a player can use to control their character.
#[derive(Clone, Debug)]
pub struct ControlScheme {
  pub id: ControlSchemeId,
  pub left: KeyCode,
  pub right: KeyCode,
  pub action: KeyCode,
}

impl ControlScheme {
  pub fn new(id: ControlSchemeId, left: KeyCode, right: KeyCode, action: KeyCode) -> Self {
    Self {
      id,
      left,
      right,
      action,
    }
  }
}

/// Represents a player that has registered to play the game. Used during the game loop.
#[derive(Clone)]
pub struct RegisteredPlayer {
  pub id: PlayerId,
  pub input: ControlScheme,
  pub colour: Color,
  pub alive: bool,
  mutable: bool,
}

impl RegisteredPlayer {
  pub fn new_mutable(id: PlayerId, input: ControlScheme, colour: Color) -> Self {
    Self {
      id,
      input,
      colour,
      alive: true,
      mutable: true,
    }
  }

  #[cfg(feature = "online")]
  pub fn new_immutable(id: PlayerId, input: ControlScheme, colour: Color) -> Self {
    Self {
      id,
      input,
      colour,
      alive: true,
      mutable: false,
    }
  }

  pub fn is_remote(&self) -> bool {
    !self.mutable
  }

  pub fn is_local(&self) -> bool {
    self.mutable
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  impl ControlScheme {
    pub(crate) fn test(id: u8) -> Self {
      Self {
        id: ControlSchemeId(id),
        left: KeyCode::ArrowLeft,
        right: KeyCode::ArrowRight,
        action: KeyCode::Space,
      }
    }
  }

  impl RegisteredPlayer {
    pub fn new_immutable_for_test(id: PlayerId, input: ControlScheme, colour: Color) -> Self {
      Self {
        id,
        input,
        colour,
        alive: true,
        mutable: false,
      }
    }

    pub fn new_mutable_dead(id: PlayerId, input: ControlScheme, colour: Color) -> Self {
      Self {
        id,
        input,
        colour,
        alive: false,
        mutable: true,
      }
    }
  }
}
