use crate::app_state::AppState;
use crate::prelude::{ConnectionInfoMessage, MenuName, ToggleMenuMessage, UiNotification};
use bevy::app::{App, Plugin, Update};
use bevy::log::{debug, error, info, warn};
use bevy::prelude::{
  Commands, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, On, Res, ResMut, in_state,
};
use mooplas_networking::prelude::{NetworkErrorEvent, NetworkRole};
use mooplas_networking_renet::prelude::{
  ClientHandshakeOutcomeMessage, ClientRenetPlugin, ClientVisualiserPlugin, RenetNetworkingMessagesPlugin,
  ServerRenetPlugin, ServerVisualiserPlugin, create_client, create_server, remove_all_renet_resources,
};

/// Plugin that adds online multiplayer capabilities for native builds using UDP/`bevy_renet` to the game.
/// Mutually exclusive with the [`crate::online::matchbox::MatchboxPlugin`].
pub struct RenetPlugin;

impl Plugin for RenetPlugin {
  fn build(&self, app: &mut App) {
    info!("Online multiplayer using [bevy_renet] is enabled");
    app
      .add_plugins(RenetNetworkingMessagesPlugin)
      .add_plugins((
        ClientRenetPlugin,
        ClientVisualiserPlugin,
        ServerRenetPlugin,
        ServerVisualiserPlugin,
      ))
      .add_systems(Update, handle_toggle_menu_message.run_if(in_state(AppState::Preparing)))
      .add_systems(
        Update,
        handle_connection_info_message
          .run_if(in_state(AppState::Preparing))
          .run_if(|network_role: Res<NetworkRole>| network_role.is_client()),
      )
      .add_systems(
        Update,
        client_handshake_system
          .run_if(in_state(AppState::Preparing))
          .run_if(|network_role: Res<NetworkRole>| network_role.is_client()),
      )
      .add_observer(receive_network_error_event);
  }
}

fn handle_toggle_menu_message(
  mut commands: Commands,
  mut messages: MessageReader<ToggleMenuMessage>,
  mut network_role: ResMut<NetworkRole>,
  mut connection_info_message: MessageWriter<ConnectionInfoMessage>,
) {
  for message in messages.read() {
    match message.active {
      MenuName::MainMenu | MenuName::PlayOnlineMenu => *network_role = NetworkRole::None,
      MenuName::HostGameMenu => *network_role = NetworkRole::Server,
      MenuName::JoinGameMenu => *network_role = NetworkRole::Client,
    }
    match *network_role {
      NetworkRole::None => remove_all_renet_resources(&mut commands),
      NetworkRole::Server => match create_server(&mut commands) {
        Ok(connection_string) => {
          debug!("Server started with connection string [{}]", connection_string);
          connection_info_message.write(ConnectionInfoMessage { connection_string });
        }
        Err(e) => {
          error!("Failed to create server: {}", e);
          *network_role = NetworkRole::None;
        }
      },
      NetworkRole::Client => {
        debug!("Waiting for connection info to create client...");
      }
    }
    debug!("Network role set to [{:?}]", network_role);
  }
}

fn handle_connection_info_message(
  mut messages: MessageReader<ConnectionInfoMessage>,
  mut commands: Commands,
  mut ui_message: MessageWriter<UiNotification>,
) {
  for message in messages.read() {
    debug!(
      "Received [ConnectionInfoMessage] with connection string [{}], attempting to parse now...",
      message.connection_string,
    );
    if let Ok(server_address) = message.connection_string.parse() {
      match create_client(&mut commands, server_address) {
        Ok(()) => info!("Created client with connection to [{}]", server_address),
        Err(e) => {
          error!("An error occurred: {}", e.to_string());
          ui_message.write(UiNotification::error(e.to_string()));
        }
      }
    } else {
      let message = format!("Invalid server address or port: [{}]", message.connection_string);
      warn!("Failed to parse connection string: {}", message);
      ui_message.write(UiNotification::error(message));
    }
  }
}

/// System that checks whether the client completed the handshake before the deadline.
/// If the handshake did not complete in time, it cleans up the client transport and
/// emits a UI error message.
///
///
pub fn client_handshake_system(
  mut messages: MessageReader<ClientHandshakeOutcomeMessage>,
  mut ui_message: MessageWriter<UiNotification>,
) {
  for message in messages.read() {
    let reason = message
      .reason
      .as_ref()
      .expect("Handshake outcome message should always contain a reason");
    match message.has_succeeded {
      true => ui_message.write(UiNotification::info(reason.to_string())),
      false => ui_message.write(UiNotification::error(reason.to_string())),
    };
  }
}

// TODO: Implement visual feedback for the user when connection is lost
#[allow(clippy::never_loop)]
fn receive_network_error_event(
  error_event: On<NetworkErrorEvent>,
  mut commands: Commands,
  mut next_app_state: ResMut<NextState<AppState>>,
  mut network_role: ResMut<NetworkRole>,
) {
  let error = error_event.event();
  if matches!(
    error,
    &NetworkErrorEvent::RenetDisconnect(_) | &NetworkErrorEvent::NetcodeDisconnect(_)
  ) {
    info!(
      "Connection lost: [{}] - removing networking resources and setting state to [{:?}]...",
      error,
      AppState::GameOver
    );
    remove_all_renet_resources(&mut commands);
    next_app_state.set(AppState::GameOver);
    *network_role = NetworkRole::None;
    return;
  }
  error!("Networking error occurred: [{}], panicking now...", error);
  panic!("{}", error);
}
