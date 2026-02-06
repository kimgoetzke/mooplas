mod components;
pub mod constants;
mod messages;
mod resources;
mod structs;

use bevy::prelude::Res;
pub use components::*;
pub use messages::*;
pub use resources::*;
pub use structs::*;

/// Checks if there are any registered players.
pub(crate) fn has_registered_players(registered: Option<Res<RegisteredPlayers>>) -> bool {
  if let Some(registered) = registered {
    !registered.players.is_empty()
  } else {
    false
  }
}
