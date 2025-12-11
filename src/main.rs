mod app_state;
mod camera;
mod controls;
mod debug;
mod game_loop;
mod game_world;
mod initialisation;
mod loading;
mod online;
mod player;
mod shared;
mod ui;

mod prelude {
  pub use crate::shared::*;
  pub use crate::app_state::AppState;
}

#[cfg(feature = "dev")]
use crate::debug::DebugPlugin;

#[cfg(feature = "online")]
use crate::online::OnlinePlugin;

use crate::app_state::AppStatePlugin;
use crate::camera::CameraPlugin;
use crate::controls::ControlsPlugin;
use crate::game_loop::GameLoopPlugin;
use crate::game_world::GameWorldPlugin;
use crate::initialisation::InitialisationPlugin;
use crate::loading::LoadingPlugin;
use crate::player::PlayerPlugin;
use crate::prelude::*;
use crate::ui::UiPlugin;
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
      LoadingPlugin,
      InitialisationPlugin,
      PlayerPlugin,
      GameLoopPlugin,
      UiPlugin,
      ControlsPlugin,
    ));

  #[cfg(feature = "online")]
  app.add_plugins(OnlinePlugin);

  #[cfg(feature = "dev")]
  app.add_plugins(DebugPlugin);

  app.run();
}
