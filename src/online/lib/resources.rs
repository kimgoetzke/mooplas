use bevy::app::{App, Plugin};
use bevy::log::*;
use bevy::prelude::{Entity, Resource};
use bevy_renet::renet::ClientId;
use std::collections::HashMap;
use std::fmt::Debug;

/// A plugin that registers and initialises shared resources used across the entire application such as [`Settings`].
pub struct NetworkingResourcesPlugin;

impl Plugin for NetworkingResourcesPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Lobby>()
      .init_resource::<NetworkClientId>()
      .init_resource::<InputSequence>();
  }
}

#[derive(Debug, Default, Resource)]
pub(crate) struct Lobby {
  pub connected: Vec<ClientId>,
  pub players: HashMap<ClientId, Entity, InputSequence>,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct NetworkClientId(Option<ClientId>);

impl NetworkClientId {
  pub fn set(&mut self, client_id: ClientId) {
    debug!("Setting [NetworkClientId] to {}", client_id);
    self.0 = Some(client_id);
  }

  pub fn get(&self) -> Option<ClientId> {
    self.0
  }
}

#[derive(Resource, Debug, Default)]
pub(crate) struct InputSequence {
  pub current: u32,
}
