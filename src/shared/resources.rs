use crate::prelude::{AvailablePlayerConfig, PlayerId, RegisteredPlayer};
use crate::shared::NetworkAudience;
use bevy::app::{App, Plugin};
use bevy::prelude::{Reflect, ReflectResource, Resource};
#[cfg(feature = "dev")]
use bevy_inspector_egui::InspectorOptions;
#[cfg(feature = "dev")]
use bevy_inspector_egui::prelude::ReflectInspectorOptions;
use std::fmt::Display;

/// A plugin that registers and initialises shared resources used across the entire application such as [`Settings`].
pub struct SharedResourcesPlugin;

impl Plugin for SharedResourcesPlugin {
  fn build(&self, app: &mut App) {
    app
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
  /// Finds an available player configuration by its [`PlayerId`].
  /// Returns `Some(&AvailablePlayerConfig)` if found, `None` otherwise.
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
  /// [`ErrorKind::PlayerNeverRegistered`] if no player with the given [`PlayerId`] exists,
  /// or [`ErrorKind::RegistrationNotMutable`] if the player exists but their `mutable` field is `false`.
  pub fn unregister_mutable(&mut self, player_id: PlayerId) -> Result<(), ErrorKind> {
    if let Some(index) = self.players.iter().position(|p| p.id == player_id) {
      if !self.players[index].mutable {
        return Err(ErrorKind::RegistrationNotMutable(player_id));
      }
      self.players.remove(index);
      Ok(())
    } else {
      Err(ErrorKind::PlayerNeverRegistered(player_id))
    }
  }

  /// Unregisters a player by their [`PlayerId`]. Returns `Ok` if the player was removed,
  /// [`ErrorKind::PlayerNeverRegistered`] if no player with the given [`PlayerId`] exists,
  /// or [`ErrorKind::RegistrationNotImmutable`] if the player exists but their `mutable` field is `true`.
  pub fn unregister_immutable(&mut self, player_id: PlayerId) -> Result<(), ErrorKind> {
    if let Some(index) = self.players.iter().position(|p| p.id == player_id) {
      if self.players[index].mutable {
        return Err(ErrorKind::RegistrationNotImmutable(player_id));
      }
      self.players.remove(index);
      Ok(())
    } else {
      Err(ErrorKind::PlayerNeverRegistered(player_id))
    }
  }
}

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
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Default)]
pub(crate) enum NetworkRole {
  #[default]
  None,
  Server,
  Client,
}

impl From<NetworkAudience> for NetworkRole {
  fn from(audience: NetworkAudience) -> Self {
    match audience {
      NetworkAudience::Server => NetworkRole::Server,
      NetworkAudience::Client => NetworkRole::Client,
    }
  }
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
  use bevy::MinimalPlugins;

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SharedResourcesPlugin));
    app
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
}
