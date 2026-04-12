use crate::prelude::{ControlSchemeId, InputMessage, PlayerId};
use bevy::math::{Quat, Vec2};
use bevy::prelude::{Component, Resource};
use mooplas_networking::prelude::SerialisableInput;
use std::collections::HashMap;

/// A component for interpolating network-synchronised transforms, controlled by the server. Used in an attempt to
/// smoothly transition from current transform's position/rotation to target position/rotation at a defined speed.
#[derive(Component)]
pub(crate) struct NetworkTransformInterpolation {
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

impl From<&PlayerId> for mooplas_networking::prelude::PlayerId {
  fn from(value: &PlayerId) -> Self {
    mooplas_networking::prelude::PlayerId(value.0)
  }
}

impl From<PlayerId> for mooplas_networking::prelude::PlayerId {
  fn from(value: PlayerId) -> Self {
    mooplas_networking::prelude::PlayerId(value.0)
  }
}

impl Into<PlayerId> for mooplas_networking::prelude::PlayerId {
  fn into(self) -> PlayerId {
    PlayerId(self.0)
  }
}

impl From<&InputMessage> for SerialisableInput {
  fn from(value: &InputMessage) -> Self {
    match value {
      InputMessage::Move(player_id, direction) => SerialisableInput::Move(player_id.0, *direction),
      InputMessage::Action(player_id) => SerialisableInput::Action(player_id.0),
    }
  }
}

impl Into<InputMessage> for SerialisableInput {
  fn into(self) -> InputMessage {
    match self {
      SerialisableInput::Move(player_id, direction) => InputMessage::Move(PlayerId(player_id), direction),
      SerialisableInput::Action(player_id) => InputMessage::Action(PlayerId(player_id)),
    }
  }
}

impl Into<InputMessage> for &SerialisableInput {
  fn into(self) -> InputMessage {
    match self {
      &SerialisableInput::Move(player_id, direction) => InputMessage::Move(PlayerId(player_id), direction),
      &SerialisableInput::Action(player_id) => InputMessage::Action(PlayerId(player_id)),
    }
  }
}

/// A client-side resource that maps local control schemes to server-assigned player identities.
/// Only relevant in online multiplayer mode.
#[cfg(feature = "online")]
#[derive(Resource, Default, Debug)]
pub(crate) struct LocalInputMapping {
  mappings: HashMap<ControlSchemeId, PlayerId>,
}

#[cfg(feature = "online")]
impl LocalInputMapping {
  pub fn insert(&mut self, control_scheme_id: ControlSchemeId, player_id: PlayerId) {
    self.mappings.insert(control_scheme_id, player_id);
  }

  pub fn clear(&mut self) {
    self.mappings.clear();
  }

  pub fn remove(&mut self, control_scheme_id: &ControlSchemeId) {
    self.mappings.remove(control_scheme_id);
  }

  pub fn get_player_id(&self, control_scheme_id: &ControlSchemeId) -> Option<PlayerId> {
    self.mappings.get(control_scheme_id).copied()
  }
}
