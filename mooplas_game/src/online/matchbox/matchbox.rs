use crate::app_state::AppState;
use crate::prelude::{ConnectionInfoMessage, MenuName, ToggleMenuMessage, UiNotification};
use bevy::app::{App, Plugin, Update};
use bevy::log::{debug, error, info};
use bevy::prelude::{
  Commands, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, On, Res, ResMut, State, in_state,
};
use mooplas_networking::prelude::{
  ClientNetworkingActive, NetworkErrorEvent, NetworkRole, ServerNetworkingActive, SignallingServerUrl,
};
#[cfg(not(target_arch = "wasm32"))]
use mooplas_networking_matchbox::prelude::start_signaling_server;
use mooplas_networking_matchbox::prelude::{
  MatchboxClientPlugin, ServerMatchboxPlugin, generate_room_url, remove_all_matchbox_resources, resolve_room_url,
  start_socket,
};

/// Plugin that adds online multiplayer capabilities for WASM targets using websocket/`bevy_matchbox` to the game.
/// Mutually exclusive with the [`crate::online::renet::RenetPlugin`].
pub struct MatchboxPlugin;

impl Plugin for MatchboxPlugin {
  fn build(&self, app: &mut App) {
    info!("Online multiplayer using [bevy_matchbox] is enabled");
    app
      .add_plugins((ServerMatchboxPlugin, MatchboxClientPlugin))
      .add_systems(Update, handle_toggle_menu_message.run_if(in_state(AppState::Preparing)))
      .add_systems(
        Update,
        handle_connection_info_message
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
  signalling_server_url: Res<SignallingServerUrl>,
  mut connection_info_message: MessageWriter<ConnectionInfoMessage>,
  mut ui_message: MessageWriter<UiNotification>,
) {
  for message in messages.read() {
    match message.active {
      MenuName::MainMenu | MenuName::PlayOnlineMenu => *network_role = NetworkRole::None,
      MenuName::HostGameMenu => *network_role = NetworkRole::Server,
      MenuName::JoinGameMenu => *network_role = NetworkRole::Client,
    }
    match *network_role {
      NetworkRole::None => remove_all_matchbox_resources(&mut commands),
      NetworkRole::Server => {
        #[cfg(not(target_arch = "wasm32"))]
        start_signaling_server(&mut commands);
        let room_id = generate_room_url();
        let room_url = format!("{}/{}", signalling_server_url.as_str().trim_end_matches('/'), &room_id);
        let connection_info = ConnectionInfoMessage::new(room_id);
        match start_socket(&mut commands, &room_url) {
          Ok(()) => {
            debug!("Server started with room URL [{}]", room_url);
            connection_info_message.write(connection_info);
            commands.insert_resource(ServerNetworkingActive);
          }
          Err(e) => {
            error!("Failed to start socket: {}", e);
            ui_message.write(UiNotification::error(e));
            *network_role = NetworkRole::None;
          }
        }
      }
      NetworkRole::Client => debug!("Waiting for connection info to create client..."),
    }
    debug!("Network role set to [{:?}]", network_role);
  }
}

fn handle_connection_info_message(
  mut messages: MessageReader<ConnectionInfoMessage>,
  mut commands: Commands,
  signalling_server_url: Res<SignallingServerUrl>,
  mut ui_message: MessageWriter<UiNotification>,
) {
  for message in messages.read() {
    let room_url = match resolve_room_url(signalling_server_url.as_str(), &message.connection_string) {
      Ok(room_url) => room_url,
      Err(error) => {
        error!("Failed to resolve room URL from connection info: {}", error);
        ui_message.write(UiNotification::error(error));
        continue;
      }
    };
    debug!(
      "Received [ConnectionInfoMessage] with connection info [{}], resolved room URL [{}], attempting to start socket now...",
      message.connection_string, room_url,
    );

    match start_socket(&mut commands, &room_url) {
      Ok(()) => {
        info!("Created client with connection to [{}]", room_url);
        commands.insert_resource(ClientNetworkingActive);
      }
      Err(e) => {
        error!("An error occurred: {}", e);
        ui_message.write(UiNotification::error(e));
      }
    }
  }
}

// TODO: Implement an visual feedback for the user when host leaves
#[allow(clippy::never_loop)]
fn receive_network_error_event(
  error_event: On<NetworkErrorEvent>,
  mut commands: Commands,
  current_app_state: Res<State<AppState>>,
  mut next_app_state: ResMut<NextState<AppState>>,
  mut network_role: ResMut<NetworkRole>,
  mut ui_message: MessageWriter<UiNotification>,
) {
  let error = error_event.event();
  if matches!(error, &NetworkErrorEvent::Disconnect(_)) {
    let next_state = match **current_app_state {
      AppState::Preparing => AppState::Preparing,
      _ => AppState::GameOver,
    };
    info!(
      "Connection lost: [{}] - removing networking resources and setting state to [{:?}]...",
      error, next_state
    );
    remove_all_matchbox_resources(&mut commands);

    // If the connection is lost during the preparation phase, we want to stay in the preparation phase to allow the
    // user to try connecting again
    if matches!(**current_app_state, AppState::Preparing)
      && matches!(next_state, AppState::Preparing)
      && network_role.is_client()
    {
      ui_message.write(UiNotification::error(
        "Unable to establish connection - is there a typo in the room ID or URL?".to_string(),
      ));
      return;
    }

    // In all other cases, we fall back to the main menu for now
    next_app_state.set(next_state);
    *network_role = NetworkRole::None;
    return;
  }
  error!("Networking error occurred: [{}], panicking now...", error);
  panic!("{}", error);
}
