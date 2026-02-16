mod client;
mod host;
mod utils;

use crate::prelude::ClientId;
use bevy::prelude::Commands;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::PeerId;
pub use client::*;
pub use host::*;
pub use utils::*;
use uuid::Uuid;

pub fn start_socket(commands: &mut Commands) {
  let socket = MatchboxSocket::new_reliable("ws://localhost:3536/hello");
  commands.insert_resource(socket);
}

#[cfg(feature = "matchbox")]
pub type RawClientId = PeerId;

#[cfg(feature = "matchbox")]
impl Default for ClientId {
  fn default() -> Self {
    ClientId(PeerId(Uuid::from_u128(0)))
  }
}
