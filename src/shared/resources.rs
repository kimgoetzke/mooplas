use crate::prelude::{AvailablePlayerConfig, PlayerId, RegisteredPlayer};
use bevy::app::{App, Plugin};
use bevy::log::*;
use bevy::prelude::{Reflect, ReflectResource, Resource};
#[cfg(feature = "dev")]
use bevy_inspector_egui::InspectorOptions;
#[cfg(feature = "dev")]
use bevy_inspector_egui::prelude::ReflectInspectorOptions;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::time::SystemTime;

/// A plugin that registers and initialises shared resources used across the entire application such as [`Settings`].
pub struct SharedResourcesPlugin;

impl Plugin for SharedResourcesPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Seed>()
      .init_resource::<Settings>()
      .register_type::<Settings>()
      .init_resource::<GeneralSettings>()
      .register_type::<GeneralSettings>()
      .register_type::<SpawnPoints>()
      .init_resource::<SpawnPoints>()
      .init_resource::<AvailablePlayerConfigs>()
      .init_resource::<RegisteredPlayers>()
      .init_resource::<WinnerInfo>()
      .init_resource::<NetworkRole>();
  }
}

/// The seed used for random number generation in the game.
#[derive(Resource, Reflect, Clone, Copy)]
pub struct Seed {
  seed: u64,
}

impl Default for Seed {
  fn default() -> Self {
    Self {
      seed: SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs(),
    }
  }
}

impl Seed {
  /// Gets the seed value.
  pub fn get(&self) -> u64 {
    self.seed
  }

  /// Sets the seed value.
  pub fn set(&mut self, seed: u64) {
    debug!("Setting seed to [{}]", seed);
    self.seed = seed;
  }
}

/// A resource that holds various settings that can be configured for the game. Intended for developer use only.
#[derive(Resource, Reflect, Clone, Copy)]
pub struct Settings {
  pub general: GeneralSettings,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      general: GeneralSettings::default(),
    }
  }
}

/// A resource that holds general settings, a child of the [`Settings`] resource. Intended for developer use only.
#[cfg(feature = "dev")]
#[derive(Resource, Reflect, InspectorOptions, Clone, Copy)]
#[reflect(Resource, InspectorOptions)]
pub struct GeneralSettings {
  /// Whether to display player gizmos that help debugging.
  pub display_player_gizmos: bool,
  /// Whether to enable (i.e. display) touch controls
  pub enable_touch_controls: bool,
}

/// A resource that holds general settings, a child of the [`Settings`] resource. Intended for developer use only.
#[cfg(not(feature = "dev"))]
#[derive(Resource, Reflect, Clone, Copy)]
#[reflect(Resource)]
pub struct GeneralSettings {
  /// Whether to display player gizmos that help debugging.
  pub display_player_gizmos: bool,
  /// Whether to enable (i.e. display) touch controls
  pub enable_touch_controls: bool,
}

impl Default for GeneralSettings {
  fn default() -> Self {
    Self {
      display_player_gizmos: false,
      enable_touch_controls: false,
    }
  }
}

/// A resource that holds all valid spawn points in the game world. Contains a list of (x, y, rotation) tuples.
#[derive(Resource, Reflect, Clone, Default)]
pub struct SpawnPoints {
  pub data: Vec<(f32, f32, f32)>,
}

/// A resource that holds all pre-configured player configurations available for players to choose from.
#[derive(Resource, Default)]
pub struct AvailablePlayerConfigs {
  pub(crate) configs: Vec<AvailablePlayerConfig>,
}

impl AvailablePlayerConfigs {
  /// Finds an available player configuration by its [`PlayerId`]. Returns [`Option<&AvailablePlayerConfig>`] if found,
  /// [`None`] otherwise.
  pub fn find_by_id(&self, player_id: PlayerId) -> Option<&AvailablePlayerConfig> {
    self.configs.iter().find(|config| config.id == player_id)
  }
}

/// A resource that holds information and configuration data about all players that have registered to play a round.
#[derive(Resource, Default)]
pub struct RegisteredPlayers {
  pub players: Vec<RegisteredPlayer>,
}

impl RegisteredPlayers {
  /// Gets the number of registered players.
  pub fn count(&self) -> usize {
    self.players.len()
  }

  /// Adds a new registered player.
  /// Returns `Ok` if the player was added, [`ErrorKind::PlayerAlreadyRegistered`] if a player with the same [`PlayerId`] already exists.
  pub fn register(&mut self, player: RegisteredPlayer) -> Result<(), ErrorKind> {
    let is_already_registered = self.players.iter().find(|p| p.id == player.id).is_none();
    if !is_already_registered {
      Err(ErrorKind::PlayerAlreadyRegistered(player.id))
    } else {
      self.players.push(player);
      Ok(())
    }
  }

  /// Unregisters a player by their [`PlayerId`]. Returns `Ok` if the player was removed,
  /// [`ErrorKind::PlayerNeverRegistered`] if no player with the given [`PlayerId`] exists, or
  /// [`ErrorKind::RegistrationNotMutable`] if the player exists but their `mutable` field is `false`.
  ///
  /// Use this method to unregister players that were also registered as mutable i.e. in a local game instance instead
  /// of in an online multiplayer game.
  pub fn unregister_mutable(&mut self, player_id: PlayerId) -> Result<(), ErrorKind> {
    if let Some(index) = self.players.iter().position(|p| p.id == player_id) {
      if self.players[index].is_remote() {
        return Err(ErrorKind::RegistrationNotMutable(player_id));
      }
      self.players.remove(index);
      Ok(())
    } else {
      Err(ErrorKind::PlayerNeverRegistered(player_id))
    }
  }

  /// Unregisters a player by their [`PlayerId`]. Returns `Ok` if the player was removed,
  /// [`ErrorKind::PlayerNeverRegistered`] if no player with the given [`PlayerId`] exists, or
  /// [`ErrorKind::RegistrationNotImmutable`] if the player exists but their `mutable` field is `true`.
  ///
  /// Use this method to unregister players that were also registered as immutable i.e. on other clients
  /// in an online multiplayer game instead of in a local game instance.
  #[cfg(feature = "online")]
  pub fn unregister_immutable(&mut self, player_id: PlayerId) -> Result<(), ErrorKind> {
    if let Some(index) = self.players.iter().position(|p| p.id == player_id) {
      if self.players[index].is_local() {
        return Err(ErrorKind::RegistrationNotImmutable(player_id));
      }
      self.players.remove(index);
      Ok(())
    } else {
      Err(ErrorKind::PlayerNeverRegistered(player_id))
    }
  }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ErrorKind {
  PlayerAlreadyRegistered(PlayerId),
  PlayerNeverRegistered(PlayerId),
  RegistrationNotMutable(PlayerId),
  RegistrationNotImmutable(PlayerId),
}

impl Display for ErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorKind::PlayerAlreadyRegistered(player_id) => {
        write!(f, "[Player {}] is already registered", player_id.0)
      }
      ErrorKind::PlayerNeverRegistered(player_id) => {
        write!(f, "[Player {}] was never registered", player_id.0)
      }
      ErrorKind::RegistrationNotMutable(player_id) => {
        write!(f, "[Player {}] is not mutably registered", player_id.0)
      }
      ErrorKind::RegistrationNotImmutable(player_id) => {
        write!(f, "[Player {}] is not immutably registered", player_id.0)
      }
    }
  }
}

/// A resource that holds information about the winner of the last round.
#[derive(Resource, Default)]
pub struct WinnerInfo {
  winner: Option<PlayerId>,
}

impl WinnerInfo {
  /// Gets the winner's [`PlayerId`], if there is one.
  pub fn get(&self) -> Option<PlayerId> {
    self.winner
  }

  /// Sets the winner's [`PlayerId`].
  pub fn set(&mut self, player_id: PlayerId) {
    self.winner = Some(player_id);
  }

  /// Clears the winner information.
  pub fn clear(&mut self) {
    self.winner = None;
  }
}

/// A resource that indicates the current network role of this application instance. Only relevant in online
/// multiplayer mode.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize)]
pub(crate) enum NetworkRole {
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::PlayerInput;
  use bevy::MinimalPlugins;
  use bevy::prelude::{Color, KeyCode};

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SharedResourcesPlugin));
    app
  }

  #[test]
  fn default_seed_is_non_zero() {
    let seed = Seed::default();
    assert!(seed.get() > 0);
  }

  #[test]
  fn set_updates_seed_value() {
    let mut seed = Seed::default();
    let new_seed = 12345;
    seed.set(new_seed);
    assert_eq!(seed.get(), new_seed);
  }

  #[test]
  fn set_allows_zero_seed() {
    let mut seed = Seed::default();
    seed.set(0);
    assert_eq!(seed.get(), 0);
  }

  #[test]
  fn shared_messages_plugin_does_not_panic_on_empty_app() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(SharedResourcesPlugin);
  }

  #[test]
  fn shared_resources_plugin_registers_plugins() {
    let app = setup();
    let world = app.world();

    assert!(world.contains_resource::<Settings>());
    assert!(world.contains_resource::<GeneralSettings>());
    assert!(world.contains_resource::<SpawnPoints>());
    let spawn_points = world
      .get_resource::<SpawnPoints>()
      .expect("Failed to retrieve SpawnPoints");
    assert!(spawn_points.data.is_empty());
    assert!(world.contains_resource::<AvailablePlayerConfigs>());
    assert!(world.contains_resource::<RegisteredPlayers>());
    assert!(world.contains_resource::<WinnerInfo>());
    assert!(world.contains_resource::<NetworkRole>());
  }

  #[test]
  fn find_by_id_returns_correct_config_when_id_exists() {
    let configs = vec![
      AvailablePlayerConfig {
        id: PlayerId(1),
        input: PlayerInput::new(PlayerId(1), KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowUp),
        colour: Default::default(),
      },
      AvailablePlayerConfig {
        id: PlayerId(2),
        input: PlayerInput::new(PlayerId(2), KeyCode::KeyZ, KeyCode::KeyC, KeyCode::KeyX),
        colour: Default::default(),
      },
    ];
    let available_configs = AvailablePlayerConfigs { configs };
    let result = available_configs.find_by_id(PlayerId(1));
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, PlayerId(1));
  }

  #[test]
  fn find_by_id_returns_none_when_id_does_not_exist() {
    let configs = vec![
      AvailablePlayerConfig {
        id: PlayerId(0),
        input: PlayerInput::new(PlayerId(0), KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::ArrowUp),
        colour: Default::default(),
      },
      AvailablePlayerConfig {
        id: PlayerId(4),
        input: PlayerInput::new(PlayerId(4), KeyCode::KeyZ, KeyCode::KeyC, KeyCode::KeyX),
        colour: Default::default(),
      },
    ];
    let available_configs = AvailablePlayerConfigs { configs };
    let result = available_configs.find_by_id(PlayerId(3));
    assert!(result.is_none());
  }

  #[test]
  fn find_by_id_returns_none_when_configs_are_empty() {
    let available_configs = AvailablePlayerConfigs { configs: vec![] };
    let result = available_configs.find_by_id(PlayerId(1));
    assert!(result.is_none());
  }

  #[test]
  fn register_adds_player_when_not_already_registered() {
    let mut registered_players = RegisteredPlayers::default();
    let player = RegisteredPlayer::new_immutable(PlayerId(1), PlayerInput::test(1), Color::default());
    let result = registered_players.register(player);
    assert!(result.is_ok());
    assert_eq!(registered_players.players.len(), 1);
    assert_eq!(registered_players.players[0].id, PlayerId(1));
  }

  #[test]
  fn register_returns_error_when_player_already_registered() {
    let mut registered_players = RegisteredPlayers::default();
    let player = RegisteredPlayer::new_immutable(PlayerId(1), PlayerInput::test(1), Color::default());
    registered_players
      .register(player.clone())
      .expect("Failed to registered player the first time");
    let result = registered_players.register(player);
    assert!(result.is_err());
    if let Err(ErrorKind::PlayerAlreadyRegistered(id)) = result {
      assert_eq!(id, PlayerId(1));
    } else {
      panic!("Expected PlayerAlreadyRegistered error");
    }
  }

  #[test]
  fn unregister_mutable_removes_player_when_registered_and_mutable() {
    let mut registered_players = RegisteredPlayers::default();
    let player = RegisteredPlayer::new_mutable(PlayerId(1), PlayerInput::test(1), Color::default());
    registered_players
      .register(player)
      .expect("Failed to registered player");
    let result = registered_players.unregister_mutable(PlayerId(1));
    assert!(result.is_ok());
    assert!(registered_players.players.is_empty());
  }

  #[test]
  fn unregister_mutable_returns_error_when_player_not_registered() {
    let mut registered_players = RegisteredPlayers::default();
    let result = registered_players.unregister_mutable(PlayerId(1));
    assert!(result.is_err());
    if let Err(ErrorKind::PlayerNeverRegistered(id)) = result {
      assert_eq!(id, PlayerId(1));
    } else {
      panic!("Expected PlayerNeverRegistered error");
    }
  }

  #[test]
  fn unregister_mutable_returns_error_when_player_is_remote() {
    let mut registered_players = RegisteredPlayers::default();
    let player = RegisteredPlayer::new_immutable(PlayerId(1), PlayerInput::test(1), Color::default());
    registered_players
      .register(player)
      .expect("Failed to registered player");
    let result = registered_players.unregister_mutable(PlayerId(1));
    assert!(result.is_err());
    if let Err(ErrorKind::RegistrationNotMutable(id)) = result {
      assert_eq!(id, PlayerId(1));
    } else {
      panic!("Expected RegistrationNotMutable error");
    }
  }

  #[cfg(feature = "online")]
  #[test]
  fn unregister_immutable_removes_player_when_registered_and_immutable() {
    let mut registered_players = RegisteredPlayers::default();
    let player = RegisteredPlayer::new_immutable(PlayerId(1), PlayerInput::test(1), Color::default());
    registered_players
      .register(player)
      .expect("Failed to registered player");
    let result = registered_players.unregister_immutable(PlayerId(1));
    assert!(result.is_ok());
    assert!(registered_players.players.is_empty());
  }

  #[cfg(feature = "online")]
  #[test]
  fn unregister_immutable_returns_error_when_player_not_registered() {
    let mut registered_players = RegisteredPlayers::default();
    let result = registered_players.unregister_immutable(PlayerId(1));
    assert!(result.is_err());
    if let Err(ErrorKind::PlayerNeverRegistered(id)) = result {
      assert_eq!(id, PlayerId(1));
    } else {
      panic!("Expected PlayerNeverRegistered error");
    }
  }

  #[cfg(feature = "online")]
  #[test]
  fn unregister_immutable_returns_error_when_player_is_local() {
    let mut registered_players = RegisteredPlayers::default();
    let player = RegisteredPlayer::new_mutable(PlayerId(1), PlayerInput::test(1), Color::default());
    registered_players
      .register(player)
      .expect("Failed to registered player");
    let result = registered_players.unregister_immutable(PlayerId(1));
    assert!(result.is_err());
    if let Err(ErrorKind::RegistrationNotImmutable(id)) = result {
      assert_eq!(id, PlayerId(1));
    } else {
      panic!("Expected RegistrationNotImmutable error");
    }
  }

  #[test]
  fn player_already_registered_error_displays_correct_message() {
    let error = ErrorKind::PlayerAlreadyRegistered(PlayerId(1));
    assert_eq!(format!("{}", error), "[Player 1] is already registered");
  }

  #[test]
  fn player_never_registered_error_displays_correct_message() {
    let error = ErrorKind::PlayerNeverRegistered(PlayerId(2));
    assert_eq!(format!("{}", error), "[Player 2] was never registered");
  }

  #[test]
  fn registration_not_mutable_error_displays_correct_message() {
    let error = ErrorKind::RegistrationNotMutable(PlayerId(3));
    assert_eq!(format!("{}", error), "[Player 3] is not mutably registered");
  }

  #[test]
  fn registration_not_immutable_error_displays_correct_message() {
    let error = ErrorKind::RegistrationNotImmutable(PlayerId(4));
    assert_eq!(format!("{}", error), "[Player 4] is not immutably registered");
  }
}
