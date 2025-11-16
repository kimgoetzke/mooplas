mod app_states;
mod camera;
mod controls;
mod game_world;
mod gizmos;
mod initialisation;
mod player;
mod shared;

mod debug {
  pub use crate::gizmos::GizmosPlugin;
  pub use bevy::input::common_conditions::input_toggle_active;
  pub use bevy_inspector_egui::bevy_egui::EguiPlugin;
  pub use bevy_inspector_egui::quick::WorldInspectorPlugin;
}

mod prelude {
  pub use crate::shared::*;
}

#[cfg(debug_assertions)]
use debug::*;

use crate::app_states::AppStatePlugin;
use crate::camera::CameraPlugin;
use crate::controls::ControlsPlugin;
use crate::game_world::GameWorldPlugin;
use crate::initialisation::InitialisationPlugin;
use crate::player::PlayerPlugin;
use crate::prelude::*;
use avian2d::PhysicsPlugins;
use avian2d::prelude::Gravity;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

fn main() {
  let mut app = App::new();
  app
    .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
    .add_plugins((
      FrameTimeDiagnosticsPlugin::default(),
      PhysicsPlugins::default().with_length_unit(5.0),
    ))
    .insert_resource(Gravity::ZERO)
    .add_plugins((
      CameraPlugin,
      AppStatePlugin,
      GameWorldPlugin,
      SharedResourcesPlugin,
      SharedMessagesPlugin,
      InitialisationPlugin,
      PlayerPlugin,
      ControlsPlugin,
    ));

  #[cfg(debug_assertions)]
  app
    .add_plugins(EguiPlugin::default())
    .add_plugins(WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F1)))
    .add_plugins(GizmosPlugin);

  app.run();
}
