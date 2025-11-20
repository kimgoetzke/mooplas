mod app_states;
mod camera;
mod controls;
mod debug;
mod game_loop;
mod game_world;
mod in_game_ui;
mod initialisation;
mod player;
mod shared;
mod touch_controls_ui;

mod prelude {
  pub use crate::shared::*;
}

#[cfg(debug_assertions)]
use crate::debug::DebugPlugin;

#[cfg(feature = "online-multiplayer")]
use bevy_renet::RenetServerPlugin;

use crate::app_states::AppStatePlugin;
use crate::camera::CameraPlugin;
use crate::controls::ControlsPlugin;
use crate::game_loop::GameLoopPlugin;
use crate::game_world::GameWorldPlugin;
use crate::in_game_ui::InGameUiPlugin;
use crate::initialisation::InitialisationPlugin;
use crate::player::PlayerPlugin;
use crate::prelude::*;
use crate::touch_controls_ui::TouchControlsUiPlugin;
use avian2d::PhysicsPlugins;
use avian2d::prelude::Gravity;
use bevy::prelude::*;

fn main() {
  let mut app = App::new();
  app
    .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
    .add_plugins((PhysicsPlugins::default().with_length_unit(5.0),))
    .insert_resource(Gravity::ZERO)
    .add_plugins((
      CameraPlugin,
      AppStatePlugin,
      GameWorldPlugin,
      SharedResourcesPlugin,
      SharedMessagesPlugin,
      InitialisationPlugin,
      PlayerPlugin,
      GameLoopPlugin,
      InGameUiPlugin,
      ControlsPlugin,
      TouchControlsUiPlugin,
    ));

  #[cfg(feature = "online-multiplayer")]
  app.add_plugins(RenetServerPlugin);

  #[cfg(debug_assertions)]
  app.add_plugins(DebugPlugin);

  app.run();
}
