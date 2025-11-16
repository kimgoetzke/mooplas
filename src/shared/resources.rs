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
      .init_resource::<SpawnPoints>();
  }
}

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

#[derive(Resource, Reflect, InspectorOptions, Clone, Copy)]
#[reflect(Resource, InspectorOptions)]
pub struct GeneralSettings {
  /// Whether to display player gizmos that help debugging.
  pub display_player_gizmos: bool,
}

impl Default for GeneralSettings {
  fn default() -> Self {
    Self {
      display_player_gizmos: true,
    }
  }
}

#[derive(Resource, Reflect, Clone, Default)]
pub struct SpawnPoints {
  pub points: Vec<(f32, f32)>,
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
    assert!(spawn_points.points.is_empty());
  }
}
