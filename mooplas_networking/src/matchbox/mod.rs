mod client;
mod host;

use bevy::prelude::Commands;
use bevy_matchbox::MatchboxSocket;
pub use client::*;
pub use host::*;

pub fn start_socket(commands: &mut Commands) {
  let socket = MatchboxSocket::new_reliable("ws://localhost:3536/hello");
  commands.insert_resource(socket);
}
