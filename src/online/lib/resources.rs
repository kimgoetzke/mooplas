use crate::prelude::InputAction;
use avian2d::math::Scalar;
use bevy::app::{App, Plugin};
use bevy::prelude::{Component, Entity, Message, Resource};
use bevy_renet::renet::ClientId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A plugin that registers and initialises shared resources used across the entire application such as [`Settings`].
pub struct NetworkingResourcesPlugin;

impl Plugin for NetworkingResourcesPlugin {
  fn build(&self, app: &mut App) {
    app.init_resource::<Lobby>().add_message::<SerialisableInputAction>();
  }
}

#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) enum SerialisableInputAction {
  Move(u8, Scalar),
  Action(u8),
}

impl Default for SerialisableInputAction {
  fn default() -> Self {
    SerialisableInputAction::Move(0, 0.0)
  }
}

impl From<&InputAction> for SerialisableInputAction {
  fn from(value: &InputAction) -> Self {
    match value {
      InputAction::Move(player_id, direction) => SerialisableInputAction::Move(player_id.0, *direction),
      InputAction::Action(player_id) => SerialisableInputAction::Action(player_id.0),
    }
  }
}

#[derive(Debug, Component)]
pub(crate) struct OnlinePlayer {
  pub id: ClientId,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct Lobby {
  pub connected: Vec<ClientId>,
  pub players: HashMap<ClientId, Entity>,
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub(crate) enum ServerMessages {
  ClientConnected { client_id: ClientId },
  ClientDisconnected { client_id: ClientId },
  StateChanged { new_state: String },
  PlayerRegistered { client_id: ClientId, player_id: u8 },
  PlayerUnregistered { client_id: ClientId, player_id: u8 },
}
