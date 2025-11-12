mod camera;
mod constants;
mod controls;
mod game_world;
mod gizmos_plugin;
mod player;
mod shared;
mod states;

use crate::camera::CameraPlugin;
use crate::controls::ControlsPlugin;
use crate::game_world::GameWorldPlugin;
use crate::gizmos_plugin::GizmosPlugin;
use crate::player::PlayerPlugin;
use crate::shared::{SharedMessagesPlugin, SharedResourcesPlugin};
use crate::states::AppStatePlugin;
use avian2d::PhysicsPlugins;
use avian2d::prelude::Gravity;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
  App::new()
    .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
    .add_plugins(EguiPlugin::default())
    .add_plugins(WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F1)))
    .add_plugins((
      FrameTimeDiagnosticsPlugin::default(),
      PhysicsPlugins::default().with_length_unit(5.0),
    ))
    .insert_resource(Gravity::ZERO)
    .add_plugins((
      CameraPlugin,
      AppStatePlugin,
      GizmosPlugin,
      GameWorldPlugin,
      SharedResourcesPlugin,
      SharedMessagesPlugin,
      PlayerPlugin,
      ControlsPlugin,
    ))
    .run();
}
