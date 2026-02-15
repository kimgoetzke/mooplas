mod client;
mod host;
mod utils;

use bevy::prelude::Commands;
use bevy_matchbox::MatchboxSocket;
pub use client::*;
pub use host::*;
pub use utils::*;

pub fn start_socket(commands: &mut Commands) {
  let socket = MatchboxSocket::new_reliable("ws://localhost:3536/hello");
  commands.insert_resource(socket);
}
