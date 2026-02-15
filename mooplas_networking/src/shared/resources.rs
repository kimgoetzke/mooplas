use bevy::app::{App, Plugin};
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

pub struct NetworkingResourcesPlugin;

impl Plugin for NetworkingResourcesPlugin {
  fn build(&self, app: &mut App) {
    app.init_resource::<NetworkRole>();
  }
}

/// A resource that indicates the current network role of this application instance. Only relevant in online
/// multiplayer mode.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize)]
pub enum NetworkRole {
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

/// Marker resource inserted when a Renet server is active. The intention is to use this for running systems
/// conditionally e.g. `.run_if(resource_exists::<ServerNetworkingActive>)`.
#[derive(Resource, Default)]
pub struct ServerNetworkingActive;

/// Marker resource inserted when a Renet client is active. The intention is to use this for running systems
/// conditionally e.g. `.run_if(resource_exists::<ClientNetworkingActive>)`.
#[derive(Resource, Default)]
pub struct ClientNetworkingActive;
