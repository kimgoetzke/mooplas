use crate::prelude::{ClientId, PlayerId};
use bevy::app::{App, Plugin};
use bevy::log::debug;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A plugin that registers and initialises shared resources used in either the client or server, or both.
pub struct NetworkingResourcesPlugin;

impl Plugin for NetworkingResourcesPlugin {
  fn build(&self, app: &mut App) {
    app.init_resource::<NetworkRole>().init_resource::<Lobby>();
  }
}

/// A resource that indicates the current network role of this application instance. Only relevant in online
/// multiplayer mode.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize)]
pub enum NetworkRole {
  #[default]
  None,
  Server,
  Client,
}

impl NetworkRole {
  /// Checks if the current role is `Server`.
  pub fn is_server(&self) -> bool {
    *self == NetworkRole::Server
  }

  /// Checks if the current role is `Client`.
  pub fn is_client(&self) -> bool {
    *self == NetworkRole::Client
  }

  pub fn is_none(&self) -> bool {
    *self == NetworkRole::None
  }
}

/// Marker resource inserted when a Renet server is active. The intention is to use this for running systems
/// conditionally e.g. `.run_if(resource_exists::<ServerNetworkingActive>)`.
#[derive(Resource, Default)]
pub struct ServerNetworkingActive;

/// Marker resource inserted when a Renet client is active. The intention is to use this for running systems
/// conditionally e.g. `.run_if(resource_exists::<ClientNetworkingActive>)`.
#[derive(Resource, Default)]
pub struct ClientNetworkingActive;

/// A resource for the server to store information about connected clients and their registered players.
#[derive(Debug, Default, Resource)]
pub struct Lobby {
  pub connected: Vec<ClientId>,
  pub registered: HashMap<ClientId, Vec<PlayerId>>,
}

impl Lobby {
  /// Registers a player for the given client ID.
  pub fn register_player(&mut self, client_id: ClientId, player_id: PlayerId) {
    self
      .registered
      .entry(client_id)
      .or_insert_with(Vec::new)
      .push(player_id);
  }

  /// Unregisters a player for the given client ID.
  pub fn unregister_player(&mut self, client_id: ClientId, player_id: PlayerId) {
    if let Some(players) = self.registered.get_mut(&client_id) {
      players.retain(|&id| id != player_id);
      if players.is_empty() {
        self.registered.remove(&client_id);
      }
    }
  }

  /// Returns a cloned list of registered players for the given client ID.
  pub fn get_registered_players_cloned(&self, client_id: &ClientId) -> Vec<PlayerId> {
    self.registered.get(client_id).cloned().unwrap_or(Vec::new())
  }

  pub fn get_client_id_by_player_id(&self, player_id: &PlayerId) -> Option<ClientId> {
    self.registered.iter().find_map(|(client_id, player_ids)| {
      if player_ids.contains(player_id) {
        Some(*client_id)
      } else {
        debug!("Player ID {} not found for client ID {}", player_id.0, client_id);
        None
      }
    })
  }

  /// Validates if the given player ID is registered for the given client ID. Returns `true` if a player is registered
  /// for the client, `false` otherwise.
  pub fn validate_registration(&self, client_id: &ClientId, player_id: &PlayerId) -> bool {
    if let Some(players) = self.registered.get(client_id) {
      players.contains(player_id)
    } else {
      false
    }
  }

  pub fn clear(&mut self) {
    self.connected.clear();
    self.registered.clear();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  impl ClientId {
    fn default_test() -> Self {
      ClientId::nil()
    }
  }

  #[test]
  fn registers_player_for_client_id() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let player_id = PlayerId(42);
    lobby.register_player(client_id, player_id);
    assert_eq!(lobby.registered.get(&client_id), Some(&vec![player_id]));
  }

  #[test]
  fn unregisters_player_for_client_id() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let player_id = PlayerId(42);
    lobby.register_player(client_id, player_id);
    lobby.unregister_player(client_id, player_id);
    assert!(lobby.registered.get(&client_id).is_none());
  }

  #[test]
  fn unregisters_player_does_not_remove_other_players() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let player_id1 = PlayerId(42);
    let player_id2 = PlayerId(43);
    lobby.register_player(client_id, player_id1);
    lobby.register_player(client_id, player_id2);
    lobby.unregister_player(client_id, player_id1);
    assert_eq!(lobby.registered.get(&client_id), Some(&vec![player_id2]));
  }

  #[test]
  fn get_registered_players_cloned_returns_empty_vec_for_unknown_client() {
    let lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let players = lobby.get_registered_players_cloned(&client_id);
    assert!(players.is_empty());
  }

  #[test]
  fn get_registered_players_cloned_returns_all_players_for_client() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let player_id1 = PlayerId(42);
    let player_id2 = PlayerId(43);
    lobby.register_player(client_id, player_id1);
    lobby.register_player(client_id, player_id2);
    let players = lobby.get_registered_players_cloned(&client_id);
    assert_eq!(players, vec![player_id1, player_id2]);
  }

  #[test]
  fn validate_registration_returns_true_for_registered_player() {
    let mut lobby = Lobby::default();
    let client_id = test_client_id(6);
    let player_id = PlayerId(42);
    lobby.register_player(client_id, player_id);
    assert!(lobby.validate_registration(&client_id, &player_id));
  }

  #[test]
  fn validate_registration_returns_false_for_unregistered_player() {
    let lobby = Lobby::default();
    let client_id = test_client_id(6);
    let player_id = PlayerId(42);
    assert!(!lobby.validate_registration(&client_id, &player_id));
  }

  #[test]
  fn validate_registration_returns_false_for_different_client() {
    let mut lobby = Lobby::default();
    let registered_client = test_client_id(1);
    let other_client = test_client_id(2);
    let registered_player = PlayerId(42);
    lobby.register_player(registered_client, registered_player);
    assert!(!lobby.validate_registration(&other_client, &registered_player));
  }

  #[test]
  fn validate_registration_returns_false_for_different_player() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let registered_player = PlayerId(42);
    let other_player = PlayerId(43);
    lobby.register_player(client_id, registered_player);
    assert!(!lobby.validate_registration(&client_id, &other_player));
  }

  #[test]
  fn validate_registration_returns_false_after_player_is_unregistered() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let player_id_1 = PlayerId(42);
    let player_id_2 = PlayerId(43);
    lobby.register_player(client_id, player_id_1);
    lobby.register_player(client_id, player_id_2);
    lobby.unregister_player(client_id, player_id_1);
    assert!(!lobby.validate_registration(&client_id, &player_id_1));
    assert!(lobby.validate_registration(&client_id, &player_id_2));
  }

  fn test_client_id(value: u128) -> ClientId {
    ClientId::from_renet_u64(value as u64)
  }
}
