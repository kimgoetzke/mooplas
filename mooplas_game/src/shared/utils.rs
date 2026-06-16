use crate::prelude::{PlayerId, RegisteredPlayers};
use bevy::color::Color;
use bevy::color::palettes::tailwind;
use bevy::prelude::Res;
use rand::RngExt;

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
    7 => Color::from(tailwind::ORANGE_500),
    _ => Color::WHITE,
  }
}

/// Word list for random name generation. Each word is at most 6 characters to fit the
/// "{Word} {d}" format within the 8-character limit.
const NAME_WORDS: &[&str] = &[
  "Potato", "Mango", "Pickle", "Waffle", "Noodle", "Turnip", "Cookie", "Pepper", "Cheese", "Breeze", "Bumble",
  "Cuddle", "Doodle", "Fudge", "Muffin", "Pebble", "Rascal", "Sprout", "Wobble", "Zigzag", "Bubble", "Crunch", "Flop",
  "Groove", "Jumble", "Nibble", "Rumble", "Snappy", "Tangle", "Wombat",
];

/// Generates a random player name in the format "{Word} {d}" where d is a digit 1-9.
/// The result is guaranteed to be at most 8 characters.
pub fn generate_random_name() -> String {
  let mut rng = rand::rng();
  let word = NAME_WORDS[rng.random_range(0..NAME_WORDS.len())];
  let digit = rng.random_range(1..=9);
  format!("{} {}", word, digit)
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

  #[test]
  fn generate_random_name_is_at_most_8_characters() {
    for _ in 0..100 {
      let name = generate_random_name();
      assert!(
        name.len() <= 8,
        "Name '{}' exceeds 8 characters (len={})",
        name,
        name.len()
      );
    }
  }

  #[test]
  fn generate_random_name_matches_word_space_digit_format() {
    for _ in 0..100 {
      let name = generate_random_name();
      let parts: Vec<&str> = name.rsplitn(2, ' ').collect();
      assert_eq!(parts.len(), 2, "Name '{}' does not match '{{Word}} {{d}}' format", name);
      let digit: u8 = parts[0].parse().expect("Last part should be a digit");
      assert!((1..=9).contains(&digit), "Digit {} not in range 1-9", digit);
      assert!(
        NAME_WORDS.contains(&parts[1]),
        "Word '{}' not in NAME_WORDS list",
        parts[1]
      );
    }
  }

  #[test]
  fn all_name_words_are_at_most_6_characters() {
    for word in NAME_WORDS {
      assert!(
        word.len() <= 6,
        "Word '{}' exceeds 6 characters (len={})",
        word,
        word.len()
      );
    }
  }
}
