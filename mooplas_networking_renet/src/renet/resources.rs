use bevy::prelude::{Commands, Deref, DerefMut, Resource};
use bevy_renet::RenetClient;
use bevy_renet::netcode::NetcodeClientTransport;
use mooplas_networking::prelude::{ClientId, ClientNetworkingActive};
use renet_visualizer::{RenetClientVisualizer, RenetServerVisualizer};
use std::time::{Duration, Instant};

/// The timeout duration (in seconds) for the client handshake process to complete before considering it as having
/// failed.
#[allow(unused)]
pub(crate) const CLIENT_HAND_SHAKE_TIMEOUT_SECS: u64 = 7;

/// Whether to show the renet visualisers by default.
pub(crate) const SHOW_VISUALISERS_BY_DEFAULT: bool = true;

/// The number of values to display in the renet visualiser graphs.
pub(crate) const VISUALISER_DISPLAY_VALUES: usize = 200;

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenetClientVisualiser(RenetClientVisualizer<{ VISUALISER_DISPLAY_VALUES }>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct RenetServerVisualiser(RenetServerVisualizer<{ VISUALISER_DISPLAY_VALUES }>);

impl RenetServerVisualiser {
  pub fn add_client(&mut self, client_id: &ClientId) {
    self.0.add_client(client_id.0);
  }

  pub fn remove_client(&mut self, client_id: &ClientId) {
    self.0.remove_client(client_id.0);
  }
}

/// Resource used to track the handshake deadline for a client connection. Used to trigger actions if a client was
/// created but did not complete the handshake in time.
#[derive(Resource)]
pub struct PendingClientHandshake {
  pub(crate) deadline: Instant,
}

impl PendingClientHandshake {
  pub fn new() -> Self {
    Self {
      deadline: Instant::now() + Duration::from_secs(CLIENT_HAND_SHAKE_TIMEOUT_SECS),
    }
  }

  pub fn clean_up_after_failure(&self, commands: &mut Commands) {
    commands.remove_resource::<RenetClient>();
    commands.remove_resource::<NetcodeClientTransport>();
    commands.remove_resource::<PendingClientHandshake>();
    commands.remove_resource::<RenetClientVisualiser>();
    commands.remove_resource::<ClientNetworkingActive>();
  }
}
