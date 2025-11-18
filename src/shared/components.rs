use bevy::color::Color;
use bevy::prelude::Component;
use std::fmt::{Debug, Display};

/// A marker component for the player entity.
#[derive(Component)]
pub struct Player;

/// A marker component for the player entity.
#[derive(Component)]
pub struct SnakeHead;

/// A marker component for entities that should wrap around the screen edges.
#[derive(Component)]
pub struct WrapAroundEntity;

/// A component identifying a player. Used to link player entities together.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct PlayerId(pub u8);

impl Display for PlayerId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Player {}", self.0)
  }
}

impl Debug for PlayerId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Player {}", self.0)
  }
}

/// The snake tail component that manages all [`crate::player::SnakeSegment`]s and sampling.
#[derive(Component)]
pub struct SnakeTail {
  pub segments: Vec<crate::player::SnakeSegment>,
  pub distance_since_last_sample: f32,
  pub gap_samples_remaining: usize,
  pub colour: Color,
}

impl Default for SnakeTail {
  fn default() -> Self {
    Self {
      segments: vec![crate::player::SnakeSegment::default()],
      distance_since_last_sample: 0.0,
      gap_samples_remaining: 0,
      colour: Color::default(),
    }
  }
}

impl SnakeTail {
  pub fn new(colour: Color) -> Self {
    Self {
      segments: vec![crate::player::SnakeSegment::default()],
      distance_since_last_sample: 0.0,
      gap_samples_remaining: 0,
      colour,
    }
  }
}
