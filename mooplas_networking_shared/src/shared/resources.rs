use crate::prelude::{ClientId, PlayerId};
use bevy::app::{App, Plugin};
use bevy::log::debug;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// A plugin that registers and initialises shared resources used in either the client or server, or both.
pub struct NetworkingResourcesPlugin;

impl Plugin for NetworkingResourcesPlugin {
  fn build(&self, app: &mut App) {
    app.init_resource::<Lobby>().init_resource::<SignallingServerUrl>();
  }
}

const DEFAULT_SIGNALLING_SERVER_URL: &str = "ws://localhost:3536";
const SIGNALLING_SERVER_URL_ENV_VAR: &str = "SIGNALLING_SERVER_URL";

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

/// A resource containing the URL of the signalling server. Loaded from the environment by default.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct SignallingServerUrl(String);

impl Default for SignallingServerUrl {
  fn default() -> Self {
    Self::from_build_time_env(option_env!("SIGNALLING_SERVER_URL"))
  }
}

impl SignallingServerUrl {
  pub fn new(url: impl Into<String>) -> Self {
    Self::try_new(url).unwrap_or_else(|error| panic!("Invalid signalling server URL: {error}"))
  }

  pub fn try_new(url: impl Into<String>) -> Result<Self, String> {
    let url = url.into();
    validate_signalling_server_base_url(&url)?;
    Ok(Self(url.trim_end_matches('/').to_string()))
  }

  fn from_build_time_env(configured_url: Option<&str>) -> Self {
    match configured_url {
      Some(url) => Self::from_build_time_value(url),
      None => Self::new(DEFAULT_SIGNALLING_SERVER_URL),
    }
  }

  fn from_build_time_value(url: &str) -> Self {
    let url = url.trim();
    assert!(
      !url.is_empty(),
      "{SIGNALLING_SERVER_URL_ENV_VAR} must not be empty when set"
    );
    Self::try_new(url).unwrap_or_else(|error| panic!("{SIGNALLING_SERVER_URL_ENV_VAR} is invalid: {error}"))
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }
}

fn validate_signalling_server_base_url(url: &str) -> Result<(), String> {
  let parsed_url = Url::parse(url).map_err(|error| format!("URL is not valid: {error}"))?;
  if !matches!(parsed_url.scheme(), "ws" | "wss") {
    return Err("URL must start with ws:// or wss://".to_string());
  }
  if parsed_url.host_str().is_none() {
    return Err("URL must include a host".to_string());
  }
  if parsed_url.scheme() == "ws" && parsed_url.port().is_none() {
    return Err("URL must include a port number (e.g., :3536)".to_string());
  }
  if parsed_url.query().is_some() || parsed_url.fragment().is_some() {
    return Err("URL must not include a query string or fragment".to_string());
  }
  Ok(())
}

/// A resource for the server to store information about connected clients and their registered players.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisteredClientPlayer {
  pub player_id: PlayerId,
  pub control_scheme_id: u8,
}

/// A resource for the server to store information about connected clients and their registered players.
#[derive(Debug, Default, Resource)]
pub struct Lobby {
  pub connected: Vec<ClientId>,
  pub registered: HashMap<ClientId, Vec<RegisteredClientPlayer>>,
}

impl Lobby {
  /// Registers a player for the given client ID.
  pub fn register_player(&mut self, client_id: ClientId, player_id: PlayerId, control_scheme_id: u8) {
    self
      .registered
      .entry(client_id)
      .or_default()
      .push(RegisteredClientPlayer {
        player_id,
        control_scheme_id,
      });
  }

  /// Unregisters a player for the given client ID.
  pub fn unregister_player(&mut self, client_id: ClientId, player_id: PlayerId) {
    if let Some(players) = self.registered.get_mut(&client_id) {
      players.retain(|registration| registration.player_id != player_id);
      if players.is_empty() {
        self.registered.remove(&client_id);
      }
    }
  }

  /// Returns a cloned list of registered players for the given client ID.
  pub fn get_registered_players_cloned(&self, client_id: &ClientId) -> Vec<PlayerId> {
    self
      .registered
      .get(client_id)
      .map(|registrations| {
        registrations
          .iter()
          .map(|registration| registration.player_id)
          .collect()
      })
      .unwrap_or_default()
  }

  /// Returns `true` if the control scheme ID is registered for the given client ID, `false` otherwise.
  pub fn is_control_scheme_registered(&self, client_id: &ClientId, control_scheme_id: u8) -> bool {
    self
      .registered
      .get(client_id)
      .map(|registrations| {
        registrations
          .iter()
          .any(|registration| registration.control_scheme_id == control_scheme_id)
      })
      .unwrap_or(false)
  }

  pub fn get_client_id_by_player_id(&self, player_id: &PlayerId) -> Option<ClientId> {
    self.registered.iter().find_map(|(client_id, registrations)| {
      if registrations
        .iter()
        .any(|registration| registration.player_id == *player_id)
      {
        Some(*client_id)
      } else {
        debug!("{} not found for client ID {}", player_id, client_id);
        None
      }
    })
  }

  /// Validates if the given player ID is registered for the given client ID. Returns `true` if a player is registered
  /// for the client, `false` otherwise.
  pub fn validate_registration(&self, client_id: &ClientId, player_id: &PlayerId) -> bool {
    if let Some(players) = self.registered.get(client_id) {
      players.iter().any(|registration| registration.player_id == *player_id)
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
  use bevy::MinimalPlugins;

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
    lobby.register_player(client_id, player_id, 3);
    assert_eq!(
      lobby.registered.get(&client_id),
      Some(&vec![RegisteredClientPlayer {
        player_id,
        control_scheme_id: 3,
      }])
    );
  }

  #[test]
  fn unregisters_player_for_client_id() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let player_id = PlayerId(42);
    lobby.register_player(client_id, player_id, 1);
    lobby.unregister_player(client_id, player_id);
    assert!(!lobby.registered.contains_key(&client_id));
  }

  #[test]
  fn unregisters_player_does_not_remove_other_players() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let player_id1 = PlayerId(42);
    let player_id2 = PlayerId(43);
    lobby.register_player(client_id, player_id1, 1);
    lobby.register_player(client_id, player_id2, 2);
    lobby.unregister_player(client_id, player_id1);
    assert_eq!(
      lobby.registered.get(&client_id),
      Some(&vec![RegisteredClientPlayer {
        player_id: player_id2,
        control_scheme_id: 2,
      }])
    );
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
    lobby.register_player(client_id, player_id1, 1);
    lobby.register_player(client_id, player_id2, 2);
    let players = lobby.get_registered_players_cloned(&client_id);
    assert_eq!(players, vec![player_id1, player_id2]);
  }

  #[test]
  fn is_control_scheme_registered_returns_true_for_registered_control_scheme() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    lobby.register_player(client_id, PlayerId(42), 2);
    assert!(lobby.is_control_scheme_registered(&client_id, 2));
    assert!(!lobby.is_control_scheme_registered(&client_id, 1));
  }

  #[test]
  fn validate_registration_returns_true_for_registered_player() {
    let mut lobby = Lobby::default();
    let client_id = test_client_id(6);
    let player_id = PlayerId(42);
    lobby.register_player(client_id, player_id, 0);
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
    lobby.register_player(registered_client, registered_player, 0);
    assert!(!lobby.validate_registration(&other_client, &registered_player));
  }

  #[test]
  fn validate_registration_returns_false_for_different_player() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let registered_player = PlayerId(42);
    let other_player = PlayerId(43);
    lobby.register_player(client_id, registered_player, 0);
    assert!(!lobby.validate_registration(&client_id, &other_player));
  }

  #[test]
  fn validate_registration_returns_false_after_player_is_unregistered() {
    let mut lobby = Lobby::default();
    let client_id = ClientId::default_test();
    let player_id_1 = PlayerId(42);
    let player_id_2 = PlayerId(43);
    lobby.register_player(client_id, player_id_1, 0);
    lobby.register_player(client_id, player_id_2, 1);
    lobby.unregister_player(client_id, player_id_1);
    assert!(!lobby.validate_registration(&client_id, &player_id_1));
    assert!(lobby.validate_registration(&client_id, &player_id_2));
  }

  #[test]
  fn networking_resources_plugin_initialises_signalling_server_url() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, NetworkingResourcesPlugin));
    let signalling_server_url = app
      .world()
      .get_resource::<SignallingServerUrl>()
      .expect("Failed to retrieve SignallingServerUrl");
    assert_eq!(signalling_server_url.as_str(), DEFAULT_SIGNALLING_SERVER_URL);
  }

  #[test]
  fn signalling_server_url_try_new_accepts_wss_url_without_port() {
    let signalling_server_url = SignallingServerUrl::try_new("wss://signal.example.com")
      .expect("Expected a valid wss:// signalling server URL without an explicit port");
    assert_eq!(signalling_server_url.as_str(), "wss://signal.example.com");
  }

  #[test]
  fn signalling_server_url_try_new_strips_trailing_slash() {
    let signalling_server_url = SignallingServerUrl::try_new("wss://signal.example.com/")
      .expect("Expected a valid signalling server URL to be normalised");
    assert_eq!(signalling_server_url.as_str(), "wss://signal.example.com");
  }

  #[test]
  fn signalling_server_url_try_new_rejects_non_websocket_url() {
    let error = SignallingServerUrl::try_new("https://signal.example.com")
      .expect_err("Expected a non-websocket signalling server URL to be rejected");
    assert!(error.contains("ws://"));
  }

  #[test]
  fn signalling_server_url_from_build_time_env_uses_default_when_unset() {
    let signalling_server_url = SignallingServerUrl::from_build_time_env(None);
    assert_eq!(signalling_server_url.as_str(), DEFAULT_SIGNALLING_SERVER_URL);
  }

  #[test]
  fn signalling_server_url_from_build_time_env_uses_custom_url() {
    let signalling_server_url = SignallingServerUrl::from_build_time_env(Some("wss://signal.example.com"));
    assert_eq!(signalling_server_url.as_str(), "wss://signal.example.com");
  }

  #[test]
  #[should_panic(expected = "SIGNALLING_SERVER_URL must not be empty when set")]
  fn signalling_server_url_from_build_time_env_rejects_empty_url() {
    let _ = SignallingServerUrl::from_build_time_env(Some("   "));
  }

  fn test_client_id(value: u128) -> ClientId {
    ClientId::from_renet_u64(value as u64)
  }
}
