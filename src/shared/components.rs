use crate::prelude::constants::SNAKE_LENGTH_MAX_CONTINUOUS;
use bevy::color::Color;
use bevy::math::Vec2;
use bevy::prelude::{Component, Entity};
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
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Into<u8> for PlayerId {
  fn into(self) -> u8 {
    self.0
  }
}

/// The snake tail component that manages all [`SnakeSegment`]s and sampling.
#[derive(Component)]
pub struct SnakeTail {
  pub segments: Vec<SnakeSegment>,
  pub distance_since_last_sample: f32,
  pub gap_samples_remaining: usize,
  pub colour: Color,
}

impl Default for SnakeTail {
  fn default() -> Self {
    Self {
      segments: vec![SnakeSegment::default()],
      distance_since_last_sample: 0.0,
      gap_samples_remaining: 0,
      colour: Color::default(),
    }
  }
}

impl SnakeTail {
  pub fn new(colour: Color) -> Self {
    Self {
      segments: vec![SnakeSegment::default()],
      distance_since_last_sample: 0.0,
      gap_samples_remaining: 0,
      colour,
    }
  }
}

/// A continuous drawn segment of the snake tail.
#[derive(Component)]
pub struct SnakeSegment {
  /// Sampled world positions along this segment.
  /// - Index 0 is the *oldest* position (furthest behind the head),
  /// - The last index is the *newest* position (closest to the head).
  positions: Vec<Vec2>,
  mesh_entity: Option<Entity>,
  collider_entity: Option<Entity>,
}

impl Default for SnakeSegment {
  fn default() -> Self {
    Self {
      positions: Vec::with_capacity(SNAKE_LENGTH_MAX_CONTINUOUS),
      mesh_entity: None,
      collider_entity: None,
    }
  }
}

impl SnakeSegment {
  /// Read-only accessor for sampled positions.
  pub fn positions(&self) -> &[Vec2] {
    &self.positions
  }

  pub fn push_position(&mut self, position: Vec2) {
    self.positions.push(position);
  }

  pub fn mesh_entity(&self) -> Option<Entity> {
    self.mesh_entity
  }

  pub fn set_mesh_entity(&mut self, entity: Entity) {
    self.mesh_entity = Some(entity);
  }

  pub fn collider_entity(&self) -> Option<Entity> {
    self.collider_entity
  }

  pub fn set_collider_entity(&mut self, entity: Entity) {
    self.collider_entity = Some(entity);
  }
}

// rust
#[cfg(test)]
mod tests {
  use super::*;
  use bevy::color::Color;
  use bevy::ecs::relationship::RelationshipSourceCollection;
  use bevy::math::Vec2;
  use bevy::prelude::Entity;

  #[test]
  fn snake_segment_default_is_empty() {
    let segment = SnakeSegment::default();
    assert!(segment.positions().is_empty());
    assert!(segment.mesh_entity().is_none());
    assert!(segment.collider_entity().is_none());
  }

  #[test]
  fn snake_segment_push_and_entities() {
    let mut segment = SnakeSegment::default();
    let position = Vec2::new(1.0, 2.0);
    segment.push_position(position);
    assert_eq!(segment.positions(), &[position]);

    let entity = Entity::new();
    segment.set_mesh_entity(entity);
    segment.set_collider_entity(entity);
    assert_eq!(segment.mesh_entity(), Some(entity));
    assert_eq!(segment.collider_entity(), Some(entity));
  }

  #[test]
  fn snake_tail_default_works() {
    let snake_tail = SnakeTail::default();
    assert_eq!(snake_tail.segments.len(), 1);
    assert_eq!(snake_tail.distance_since_last_sample, 0.0);
    assert_eq!(snake_tail.gap_samples_remaining, 0);
    assert_eq!(snake_tail.colour, Color::default());
  }

  #[test]
  fn snake_tail_new_works() {
    let red = Color::srgb(1.0, 0.0, 0.0);
    let tail2 = SnakeTail::new(red);
    assert_eq!(tail2.colour, red);
  }

  #[test]
  fn snake_tail_segment_push_position_works() {
    let mut tail = SnakeTail::default();
    let pos = Vec2::new(3.0, 4.0);
    tail.segments[0].push_position(pos);
    assert_eq!(tail.segments[0].positions(), &[pos]);
  }
}
