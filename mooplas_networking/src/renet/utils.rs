use crate::renet::{RenetClientVisualiser, RenetServerVisualiser};
use bevy::prelude::Commands;
use bevy_renet::netcode::{NetcodeClientTransport, NetcodeServerTransport};
use bevy_renet::{RenetClient, RenetServer};

use crate::prelude::{ClientNetworkingActive, PendingClientHandshake, ServerNetworkingActive};

/// Cleans up all networking resources for native platforms.
pub fn remove_all_renet_resources(commands: &mut Commands) {
  commands.remove_resource::<RenetServer>();
  commands.remove_resource::<NetcodeServerTransport>();
  commands.remove_resource::<RenetServerVisualiser>();
  commands.remove_resource::<ServerNetworkingActive>();
  commands.remove_resource::<RenetClient>();
  commands.remove_resource::<NetcodeClientTransport>();
  commands.remove_resource::<RenetClientVisualiser>();
  commands.remove_resource::<ClientNetworkingActive>();
  commands.remove_resource::<PendingClientHandshake>();
}
