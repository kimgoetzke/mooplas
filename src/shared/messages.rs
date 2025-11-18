use crate::shared::{PlayerId};
use bevy::app::{App, Plugin};
use bevy::prelude::Message;

/// A plugin that registers all shared messages used across multiple plugins and systems.
pub struct SharedMessagesPlugin;

impl Plugin for SharedMessagesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<DebugStateMessage>()
      .add_message::<PlayerRegistrationMessage>();
  }
}

#[allow(dead_code)]
/// A message that communicates the current state of debug related settings.
#[derive(Message)]
pub struct DebugStateMessage {
  pub display_player_gizmos: bool,
}

/// A message that communicates a change to a user's registration status in the lobby.
#[derive(Message, Debug)]
pub struct PlayerRegistrationMessage {
  pub player_id: PlayerId,
  pub has_registered: bool,
  pub is_anyone_registered: bool,
}

#[cfg(test)]
mod tests {
  use super::*;
  use bevy::MinimalPlugins;
  use bevy::prelude::Messages;

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SharedMessagesPlugin));
    app
  }

  #[test]
  fn shared_messages_plugin_does_not_panic_on_empty_app() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(SharedMessagesPlugin);
  }

  #[test]
  fn shared_messages_plugin_registers_debug_state_message() {
    let app = setup();
    assert!(app.world().contains_resource::<Messages<DebugStateMessage>>());
    assert!(app.world().contains_resource::<Messages<PlayerRegistrationMessage>>());
  }

  #[test]
  fn debug_state_message_can_be_written_and_read() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SharedMessagesPlugin));
    let message_id = app
      .world_mut()
      .write_message(DebugStateMessage {
        display_player_gizmos: true,
      })
      .unwrap()
      .id;
    let messages = app
      .world_mut()
      .get_resource_mut::<Messages<DebugStateMessage>>()
      .expect("Failed to get Messages<DebugStateMessage>");
    let message = messages.get_message(message_id).expect("Failed to get message");
    assert!(message.0.display_player_gizmos);
  }
}
