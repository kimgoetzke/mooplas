use bevy::app::{App, Plugin};
use bevy::prelude::Resource;
use bevy_renet::renet::ClientId;
use std::fmt::Debug;

/// A plugin that registers and initialises shared resources used across the entire application such as [`Settings`].
pub struct NetworkingResourcesPlugin;

impl Plugin for NetworkingResourcesPlugin {
  fn build(&self, app: &mut App) {
    app.init_resource::<Lobby>().init_resource::<InputSequence>();
  }
}

#[derive(Debug, Default, Resource)]
pub(crate) struct Lobby {
  pub connected: Vec<ClientId>,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct InputSequence {
  current: u32,
}

impl InputSequence {
  pub fn current(&self) -> u32 {
    self.current
  }

  pub fn next(&mut self) -> u32 {
    self.current = self.current.wrapping_add(1);
    self.current()
  }
}
