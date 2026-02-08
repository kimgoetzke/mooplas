use crate::prelude::PlayerId;
use bevy::app::{App, Plugin};
use bevy::prelude::{Commands, Deref, DerefMut, Resource};
use bevy_renet::RenetClient;
use bevy_renet::netcode::NetcodeClientTransport;
use bevy_renet::renet::ClientId;
use renet_visualizer::{RenetClientVisualizer, RenetServerVisualizer};
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Duration, Instant};

/// A plugin that registers and initialises shared resources used in either the client or server, or both.
pub struct NetworkingResourcesPlugin;

impl Plugin for NetworkingResourcesPlugin {
  fn build(&self, app: &mut App) {
    app.init_resource::<Lobby>();
  }
}

/// The timeout duration (in seconds) for the client handshake process to complete before considering it as having
/// failed.
#[allow(unused)]
pub(crate) const CLIENT_HAND_SHAKE_TIMEOUT_SECS: u64 = 7;

/// Whether to show the renet visualisers by default.
pub(crate) const SHOW_VISUALISERS_BY_DEFAULT: bool = true;

/// The number of values to display in the renet visualiser graphs.
pub(crate) const VISUALISER_DISPLAY_VALUES: usize = 200;

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenetClientVisualiser(RenetClientVisualizer<{ VISUALISER_DISPLAY_VALUES }>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenetServerVisualiser(RenetServerVisualizer<{ VISUALISER_DISPLAY_VALUES }>);

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

/// Resource used to track the handshake deadline for a client connection. Used to trigger actions if a client was
/// created but did not complete the handshake in time.
#[derive(Resource)]
pub struct PendingClientHandshake {
  pub deadline: Instant,
}

impl PendingClientHandshake {
  pub fn new() -> Self {
    Self {
      deadline: Instant::now() + Duration::from_secs(CLIENT_HAND_SHAKE_TIMEOUT_SECS),
    }
  }

  pub fn clean_up_after_failure(&self, commands: &mut Commands) {
    commands.remove_resource::<RenetClient>();
    commands.remove_resource::<NetcodeClientTransport>();
    commands.remove_resource::<PendingClientHandshake>();
    commands.remove_resource::<RenetClientVisualiser>()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use bevy_renet::renet::ClientId;

  #[test]
  fn registers_player_for_client_id() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default();
    let player_id = PlayerId(42);
    lobby.register_player(client_id, player_id);
    assert_eq!(lobby.registered.get(&client_id), Some(&vec![player_id]));
  }

  #[test]
  fn unregisters_player_for_client_id() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default();
    let player_id = PlayerId(42);
    lobby.register_player(client_id, player_id);
    lobby.unregister_player(client_id, player_id);
    assert!(lobby.registered.get(&client_id).is_none());
  }

  #[test]
  fn unregisters_player_does_not_remove_other_players() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default();
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
    let client_id = ClientId::default();
    let players = lobby.get_registered_players_cloned(&client_id);
    assert!(players.is_empty());
  }

  #[test]
  fn get_registered_players_cloned_returns_all_players_for_client() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default();
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
    let client_id = ClientId::from(6u64);
    let player_id = PlayerId(42);
    lobby.register_player(client_id, player_id);
    assert!(lobby.validate_registration(&client_id, &player_id));
  }

  #[test]
  fn validate_registration_returns_false_for_unregistered_player() {
    let lobby = Lobby::default();
    let client_id = ClientId::from(6u64);
    let player_id = PlayerId(42);
    assert!(!lobby.validate_registration(&client_id, &player_id));
  }

  #[test]
  fn validate_registration_returns_false_for_different_client() {
    let mut lobby = Lobby::default();
    let registered_client = ClientId::default();
    let other_client = ClientId::from(1u64);
    let registered_player = PlayerId(42);
    lobby.register_player(registered_client, registered_player);
    assert!(!lobby.validate_registration(&other_client, &registered_player));
  }

  #[test]
  fn validate_registration_returns_false_for_different_player() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default();
    let registered_player = PlayerId(42);
    let other_player = PlayerId(43);
    lobby.register_player(client_id, registered_player);
    assert!(!lobby.validate_registration(&client_id, &other_player));
  }

  #[test]
  fn validate_registration_returns_false_after_player_is_unregistered() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default();
    let player_id_1 = PlayerId(42);
    let player_id_2 = PlayerId(43);
    lobby.register_player(client_id, player_id_1);
    lobby.register_player(client_id, player_id_2);
    lobby.unregister_player(client_id, player_id_1);
    assert!(!lobby.validate_registration(&client_id, &player_id_1));
    assert!(lobby.validate_registration(&client_id, &player_id_2));
  }
}
