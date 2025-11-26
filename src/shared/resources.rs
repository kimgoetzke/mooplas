use crate::prelude::{AvailablePlayerConfig, PlayerId, RegisteredPlayer};
use bevy::app::{App, Plugin};
use bevy::prelude::{Reflect, ReflectResource, Resource};
use bevy_inspector_egui::InspectorOptions;
use bevy_inspector_egui::prelude::ReflectInspectorOptions;

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
      .init_resource::<WinnerInfo>();
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
#[derive(Resource, Reflect, InspectorOptions, Clone, Copy)]
#[reflect(Resource, InspectorOptions)]
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

/// A resource that holds information and configuration data about all players that have registered to play a round.
#[derive(Resource, Default)]
pub struct RegisteredPlayers {
  pub players: Vec<RegisteredPlayer>,
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
  }
}
