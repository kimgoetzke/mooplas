use crate::app_state::AppState;
use crate::prelude::{ConnectionInfoMessage, MenuName, ToggleMenuMessage, UiNotification};
use bevy::app::{App, Plugin, Update};
use bevy::log::{debug, error, info};
use bevy::prelude::{
  Commands, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, On, Res, ResMut, in_state,
};
use mooplas_networking::prelude::{ClientNetworkingActive, NetworkErrorEvent, NetworkRole, ServerNetworkingActive};
use mooplas_networking_matchbox::prelude::{
  MatchboxClientPlugin, ServerMatchboxPlugin, generate_room_url, remove_all_matchbox_resources, start_signaling_server,
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
        debug!("Creating server...");
        start_signaling_server(&mut commands);
        let room_url = generate_room_url();
        match start_socket(&mut commands, &room_url) {
          Ok(()) => {
            debug!("Server started with room URL [{}]", room_url);
            connection_info_message.write(ConnectionInfoMessage {
              connection_string: room_url,
            });
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
  mut ui_message: MessageWriter<UiNotification>,
) {
  for message in messages.read() {
    debug!(
      "Received [ConnectionInfoMessage] with connection string [{}], attempting to start socket now...",
      message.connection_string,
    );

    // TODO: Handle invalid connection string - currently this always succeeds and later panics
    match start_socket(&mut commands, &message.connection_string) {
      Ok(()) => {
        info!("Created client with connection to [{}]", message.connection_string);
        commands.insert_resource(ClientNetworkingActive);
      }
      Err(e) => {
        error!("An error occurred: {}", e);
        ui_message.write(UiNotification::error(e));
      }
    }
  }
}

// TODO: Implement an visual feedback for the user when connection is lost
// TODO: Handle network disconnect errors for Matchbox
#[allow(clippy::never_loop)]
fn receive_network_error_event(
  error_event: On<NetworkErrorEvent>,
  mut commands: Commands,
  mut next_app_state: ResMut<NextState<AppState>>,
  mut network_role: ResMut<NetworkRole>,
) {
  let error = error_event.event();
  if matches!(error, &NetworkErrorEvent::NetcodeDisconnect(_)) {
    info!(
      "Connection lost: [{}] - removing networking resources and setting state to [{:?}]...",
      error,
      AppState::GameOver
    );
    remove_all_matchbox_resources(&mut commands);
    next_app_state.set(AppState::GameOver);
    *network_role = NetworkRole::None;
    return;
  }
  error!("Networking error occurred: [{}], panicking now...", error);
  panic!("{}", error);
}
