use bevy::math::{Quat, Vec2};
use bevy::prelude::Component;

/// A component for interpolating network-synchronised transforms, controlled by the server. Used in an attempt to
/// smoothly transition from current transform's position/rotation to target position/rotation at a defined speed.
#[derive(Component)]
pub struct NetworkTransformInterpolation {
  /// The position to interpolate towards.
  pub target_position: Vec2,
  /// The rotation to interpolate towards.
  pub target_rotation: Quat,
  /// The speed at which to interpolate, ranging from 0.0 to 1.0 (higher = faster).
  pub interpolation_speed: f32,
}

impl NetworkTransformInterpolation {
  pub fn new(speed: f32) -> Self {
    Self {
      target_position: Vec2::ZERO,
      target_rotation: Quat::IDENTITY,
      interpolation_speed: speed.clamp(0.0, 1.0),
    }
  }

  pub fn update_target(&mut self, position: Vec2, rotation: Quat) {
    self.target_position = position;
    self.target_rotation = rotation;
  }
}
