use crate::prelude::NetworkRole;
use crate::shared::PlayerId;
use avian2d::math::Scalar;
use bevy::app::{App, Plugin};
use bevy::prelude::Message;
use serde::{Deserialize, Serialize};

#[cfg(feature = "online")]
use crate::prelude::constants::{ERROR_COLOUR, INFO_COLOUR};
#[cfg(feature = "online")]
use bevy::prelude::{Color, Srgba};

/// A plugin that registers all shared messages used across multiple plugins and systems.
pub struct SharedMessagesPlugin;

impl Plugin for SharedMessagesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<DebugStateMessage>()
      .add_message::<ToggleMenuMessage>()
      .add_message::<PlayerRegistrationMessage>()
      .add_message::<ContinueMessage>()
      .add_message::<ExitLobbyMessage>()
      .add_message::<TouchControlsToggledMessage>()
      .add_message::<InputMessage>();

    #[cfg(feature = "online")]
    app
      .add_message::<ConnectionInfoMessage>()
      .add_message::<UiNotification>();
  }
}

#[allow(dead_code)]
/// A message that communicates the current state of debug related settings.
#[derive(Message)]
pub struct DebugStateMessage {
  pub display_player_gizmos: bool,
}

/// A message that communicates a change to a user's registration status in the lobby.
#[derive(Message, Debug, Serialize, Deserialize, Copy, Clone)]
pub(crate) struct PlayerRegistrationMessage {
  pub player_id: PlayerId,
  /// Whether the player has registered (true) or unregistered (false).
  pub has_registered: bool,
  /// Whether any player is currently registered, after this change.
  pub is_anyone_registered: bool,
  /// Whether this message originated from the server or the client. Used to prevent echoing.
  pub network_role: Option<NetworkRole>,
}

/// A message that communicates a change to the touch controls setting.
#[derive(Message)]
pub struct TouchControlsToggledMessage {
  pub enabled: bool,
}

impl TouchControlsToggledMessage {
  pub fn new(enabled: bool) -> Self {
    Self { enabled }
  }
}

/// A [`Message`] indicating that the spawn menu should be opened.
#[derive(Message)]
pub struct ToggleMenuMessage {
  pub active: MenuName,
}

impl ToggleMenuMessage {
  pub fn set(active: MenuName) -> Self {
    Self { active }
  }
}

/// The name identifying a menu. Used by the [`ToggleMenuMessage`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuName {
  MainMenu,
  PlayOnlineMenu,
  HostGameMenu,
  JoinGameMenu,
}

/// A [`Message`] written for an input action by a player.
#[derive(Message, Clone, Copy, Debug)]
pub enum InputMessage {
  Move(PlayerId, Scalar),
  Action(PlayerId),
}

/// A [`Message`] indicating that the game should continue (e.g., start or restart). Used when an arbitrary player
/// input is required.
#[derive(Message)]
pub struct ContinueMessage;

/// A [`Message`] indicating that the player wants to exit the lobby.
#[derive(Message)]
pub struct ExitLobbyMessage {
  /// Whether the exit is being forced (true) by server or voluntary (false).
  #[allow(unused)]
  pub(crate) by_force: bool,
}

impl ExitLobbyMessage {
  pub fn default() -> Self {
    Self { by_force: false }
  }

  #[cfg(feature = "online")]
  pub fn forced_by_server() -> Self {
    Self { by_force: true }
  }
}

/// A [`Message`] indicating that the server connection info should be updated, wherever it may be used.
#[cfg(feature = "online")]
#[derive(Message, Clone)]
pub struct ConnectionInfoMessage {
  pub connection_string: String,
}

/// A [`Message`] for displaying an error message in the UI.
#[cfg(feature = "online")]
#[derive(Message, Clone)]
pub struct UiNotification {
  pub text: String,
  srbga: Srgba,
  reset_custom_interaction: bool,
}

#[cfg(feature = "online")]
impl UiNotification {
  pub fn error(text: String) -> Self {
    Self {
      text,
      srbga: ERROR_COLOUR,
      reset_custom_interaction: true,
    }
  }

  pub fn info(text: String) -> Self {
    Self {
      text,
      srbga: INFO_COLOUR,
      reset_custom_interaction: false,
    }
  }

  pub fn colour(&self) -> Color {
    Color::from(self.srbga)
  }

  pub fn should_reset_custom_interaction(&self) -> bool {
    self.reset_custom_interaction
  }
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
    assert!(app.world().contains_resource::<Messages<ToggleMenuMessage>>());
    assert!(app.world().contains_resource::<Messages<PlayerRegistrationMessage>>());
    assert!(app.world().contains_resource::<Messages<TouchControlsToggledMessage>>());
    assert!(app.world().contains_resource::<Messages<InputMessage>>());
    assert!(app.world().contains_resource::<Messages<ContinueMessage>>());
    assert!(app.world().contains_resource::<Messages<ExitLobbyMessage>>());
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
