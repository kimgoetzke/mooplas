mod client;
mod client_visualiser;
mod messages;
mod resources;
mod server;
mod server_visualiser;
mod utils;

pub use client::*;
pub use client_visualiser::ClientVisualiserPlugin;
pub use messages::*;
pub use resources::*;
pub use server::*;
pub use server_visualiser::ServerVisualiserPlugin;
pub use utils::*;

pub(crate) const PROTOCOL_ID: u64 = 1000;

use bevy_renet::renet::ClientId as RenetClientId;
use mooplas_networking::prelude::ClientId;

pub fn client_id_from_renet_id(value: RenetClientId) -> ClientId {
  ClientId::from_renet_u64(value)
}

pub fn renet_id_from_client_id(value: ClientId) -> RenetClientId {
  value.to_renet_u64()
}
