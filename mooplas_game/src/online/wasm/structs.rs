use crate::prelude::{InputMessage, PlayerId, PlayerRegistrationMessage};
use bevy::math::{Quat, Vec2};
use bevy::prelude::Component;
use mooplas_networking::prelude::SerialisableInputMessage;

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

impl From<&PlayerRegistrationMessage> for mooplas_networking::prelude::PlayerRegistrationMessage {
  fn from(value: &PlayerRegistrationMessage) -> Self {
    mooplas_networking::prelude::PlayerRegistrationMessage {
      player_id: value.player_id.into(),
      has_registered: value.has_registered,
      is_anyone_registered: value.is_anyone_registered,
      network_role: value.network_role,
    }
  }
}

impl Into<PlayerRegistrationMessage> for mooplas_networking::prelude::PlayerRegistrationMessage {
  fn into(self) -> PlayerRegistrationMessage {
    PlayerRegistrationMessage {
      player_id: self.player_id.into(),
      has_registered: self.has_registered,
      is_anyone_registered: self.is_anyone_registered,
      network_role: self.network_role,
    }
  }
}

impl From<&InputMessage> for SerialisableInputMessage {
  fn from(value: &InputMessage) -> Self {
    match value {
      InputMessage::Move(player_id, direction) => SerialisableInputMessage::Move(player_id.0, *direction),
      InputMessage::Action(player_id) => SerialisableInputMessage::Action(player_id.0),
    }
  }
}

impl Into<InputMessage> for SerialisableInputMessage {
  fn into(self) -> InputMessage {
    match self {
      SerialisableInputMessage::Move(player_id, direction) => InputMessage::Move(PlayerId(player_id), direction),
      SerialisableInputMessage::Action(player_id) => InputMessage::Action(PlayerId(player_id)),
    }
  }
}

impl Into<InputMessage> for &SerialisableInputMessage {
  fn into(self) -> InputMessage {
    match self {
      &SerialisableInputMessage::Move(player_id, direction) => InputMessage::Move(PlayerId(player_id), direction),
      &SerialisableInputMessage::Action(player_id) => InputMessage::Action(PlayerId(player_id)),
    }
  }
}
