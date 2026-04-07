use crate::prelude::{PlayerId, RegisteredPlayers};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::prelude::Res;

/// Checks if there are any registered players.
pub fn has_registered_players(registered: Option<Res<RegisteredPlayers>>) -> bool {
  if let Some(registered) = registered {
    !registered.players.is_empty()
  } else {
    false
  }
}

/// Deterministically maps a [`PlayerId`] to a colour using the tailwind palette. In online multiplayer, all machines
/// share the same lookup, so no colour data needs to be sent over the wire.
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::PlayerId;
  use bevy::color::Color;

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
