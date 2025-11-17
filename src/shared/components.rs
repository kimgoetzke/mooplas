use bevy::color::Color;
use bevy::prelude::Component;

/// A marker component for the player entity.
#[derive(Component)]
pub struct Player;

/// A marker component for the player entity.
#[derive(Component)]
pub struct SnakeHead;

/// A marker component for entities that should wrap around the screen edges.
#[derive(Component)]
pub struct WrapAroundEntity;

/// A component identifying which player an entity belongs to. Used to route input and per-player logic, etc.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlayerId(pub u8);

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
