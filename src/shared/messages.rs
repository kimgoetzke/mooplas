use bevy::app::{App, Plugin};
use bevy::prelude::Message;

/// A plugin that registers all shared messages used across multiple plugins and systems.
pub struct SharedMessagesPlugin;

impl Plugin for SharedMessagesPlugin {
  fn build(&self, app: &mut App) {
    app.add_message::<DebugStateMessage>();
  }
}

#[allow(dead_code)]
/// A message that communicates the current state of debug related settings.
#[derive(Message)]
pub struct DebugStateMessage {
  pub display_player_gizmos: bool,
}
