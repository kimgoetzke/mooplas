mod client;
mod host;
mod utils;

use bevy::prelude::Commands;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::PeerId;
pub use client::*;
pub use host::*;
use mooplas_networking::prelude::ClientId;
pub use utils::*;

pub fn start_socket(commands: &mut Commands) {
  let socket = MatchboxSocket::new_reliable("ws://localhost:3536/hello");
  commands.insert_resource(socket);
}

pub fn client_id_from_peer_id(peer_id: PeerId) -> ClientId {
  ClientId::from_uuid(peer_id.0)
}

pub fn peer_id_from_client_id(client_id: ClientId) -> PeerId {
  PeerId(client_id.as_uuid())
}
