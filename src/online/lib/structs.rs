use crate::online::lib::messages::SerialisableInputActionMessage;
use crate::prelude::PlayerRegistrationMessage;
use crate::shared::PlayerId;
use bevy::math::{Quat, Vec2};
use bevy::prelude::Component;
use bevy_renet::renet::ClientId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessage {
  ClientConnected {
    client_id: ClientId,
  },
  ClientDisconnected {
    client_id: ClientId,
  },
  ClientInitialised {
    seed: u64,
    client_id: ClientId,
  },
  /// Indicates that the app state has changed on the server.
  StateChanged {
    new_state: String,
    winner_info: Option<PlayerId>,
  },
  PlayerRegistered {
    client_id: ClientId,
    player_id: u8,
  },
  PlayerUnregistered {
    client_id: ClientId,
    player_id: u8,
  },
  /// Contains authoritative player state updates in a vec of (player_id, x, y, rotation).
  UpdatePlayerStates {
    states: Vec<(u8, f32, f32, f32)>,
  },
}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
  PlayerRegistration(PlayerRegistrationMessage),
  Input(u32, SerialisableInputActionMessage),
}

impl Debug for ClientMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ClientMessage::PlayerRegistration(message) => {
        write!(f, "ClientMessage::PlayerRegistration for {}", message.player_id)
      }
      ClientMessage::Input(sequence, action) => {
        write!(f, "ClientMessage::{:?} (#{})", action, sequence)
      }
    }
  }
}

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
