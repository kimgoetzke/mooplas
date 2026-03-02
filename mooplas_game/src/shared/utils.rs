use crate::prelude::RegisteredPlayers;
use bevy::prelude::Res;

/// Checks if there are any registered players.
pub fn has_registered_players(registered: Option<Res<RegisteredPlayers>>) -> bool {
  if let Some(registered) = registered {
    !registered.players.is_empty()
  } else {
    false
  }
}
