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
use uuid::Uuid;

pub fn start_socket(commands: &mut Commands) {
  let socket = MatchboxSocket::new_reliable("ws://localhost:3536/hello");
  commands.insert_resource(socket);
}

pub type RawClientId = PeerId;

// impl Default for RawClientId {
//   fn default() -> Self {
//     PeerId(Uuid::from_u128(0)) as RawClientId
//   }
// }
