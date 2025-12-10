use crate::shared::PlayerId;
use bevy::app::{App, Plugin};
use bevy::prelude::Resource;
use bevy_renet::renet::ClientId;
use std::collections::HashMap;
use std::fmt::Debug;

/// A plugin that registers and initialises shared resources used across the entire application such as [`Settings`].
pub struct NetworkingResourcesPlugin;

impl Plugin for NetworkingResourcesPlugin {
  fn build(&self, app: &mut App) {
    app.init_resource::<Lobby>().init_resource::<InputSequence>();
  }
}

#[derive(Debug, Default, Resource)]
pub(crate) struct Lobby {
  pub connected: Vec<ClientId>,
  pub registered: HashMap<ClientId, Vec<PlayerId>>,
}

impl Lobby {
  pub fn register_player(&mut self, client_id: ClientId, player_id: PlayerId) {
    self
      .registered
      .entry(client_id)
      .or_insert_with(Vec::new)
      .push(player_id);
  }

  pub fn unregister_player(&mut self, client_id: ClientId, player_id: PlayerId) {
    if let Some(players) = self.registered.get_mut(&client_id) {
      players.retain(|&id| id != player_id);
      if players.is_empty() {
        self.registered.remove(&client_id);
      }
    }
  }

  pub fn get_registered_players(&self, client_id: &ClientId) -> Vec<PlayerId> {
    self.registered.get(client_id).cloned().unwrap_or(Vec::new())
  }
}

#[derive(Resource, Debug, Default)]
pub(crate) struct InputSequence {
  current: u32,
}

impl InputSequence {
  pub fn current(&self) -> u32 {
    self.current
  }

  pub fn next(&mut self) -> u32 {
    self.current = self.current.wrapping_add(1);
    self.current()
  }
}
