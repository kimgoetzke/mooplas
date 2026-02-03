#![cfg(feature = "dev")]

mod controls;
mod gizmos;

use crate::debug::controls::DebugControlsPlugin;
use crate::debug::gizmos::GizmosPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::{App, KeyCode, Plugin};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

/// A plugin that adds various debugging tools and utilities to the application. This includes tools from third party
/// crates, such as world inspector, as well as custom debugging controls and gizmos.
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(EguiPlugin::default())
      .add_plugins(FrameTimeDiagnosticsPlugin::default())
      .add_plugins(WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F1)))
      .add_plugins((DebugControlsPlugin, GizmosPlugin));
  }
}
