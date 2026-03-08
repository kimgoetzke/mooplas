use crate::renet::{PendingClientHandshake, RenetClientVisualiser, RenetServerVisualiser};
use bevy::prelude::Commands;
use bevy_renet::netcode::{NetcodeClientTransport, NetcodeServerTransport};
use bevy_renet::renet::ClientId as RenetClientId;
use bevy_renet::{RenetClient, RenetServer};
use mooplas_networking::prelude::{ClientId, ClientNetworkingActive, ServerNetworkingActive};

/// Give it a [`RenetClientId`] and it'll return a [`ClientId`].
pub fn client_id_from_renet_id(value: RenetClientId) -> ClientId {
  ClientId::from_renet_u64(value)
}

/// Give it a [`ClientId`] and it'll return a [`RenetClientId`].
pub fn renet_id_from_client_id(value: ClientId) -> RenetClientId {
  value.to_renet_u64()
}

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
