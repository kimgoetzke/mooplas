use crate::prelude::PlayerId;
use bevy::color::palettes::tailwind;
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

/// Deterministically maps a [`PlayerId`] to a colour using the tailwind palette. All machines
/// share the same lookup so no colour data needs to be sent over the wire.
pub fn colour_for_player_id(player_id: PlayerId) -> Color {
  match player_id.0 {
    0 => Color::from(tailwind::ROSE_500),
    1 => Color::from(tailwind::LIME_500),
    2 => Color::from(tailwind::SKY_500),
    3 => Color::from(tailwind::VIOLET_500),
    4 => Color::from(tailwind::AMBER_500),
    5 => Color::from(tailwind::EMERALD_500),
    6 => Color::from(tailwind::PINK_500),
    7 => Color::from(tailwind::RED_500),
    _ => Color::WHITE,
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

  #[test]
  fn colour_for_player_id_returns_distinct_colours_for_ids_0_through_4() {
    let colours: Vec<Color> = (0..5).map(|i| colour_for_player_id(PlayerId(i))).collect();
    for i in 0..colours.len() {
      for j in (i + 1)..colours.len() {
        assert_ne!(
          colours[i], colours[j],
          "PlayerId({}) and PlayerId({}) should have distinct colours",
          i, j
        );
      }
    }
  }

  #[test]
  fn colour_for_player_id_returns_white_for_unknown_id() {
    assert_eq!(colour_for_player_id(PlayerId(255)), Color::WHITE);
  }
}
