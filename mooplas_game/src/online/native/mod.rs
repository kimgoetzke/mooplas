#![cfg(feature = "online")]

mod client;
mod server;
mod structs;
mod utils;

use crate::online::native::client::ClientPlugin;
use crate::online::native::server::ServerPlugin;
use crate::prelude::{AppState, MenuName, NetworkRole, ToggleMenuMessage, UiNotification};
use crate::shared::ConnectionInfoMessage;
use bevy::app::Update;
use bevy::log::*;
use bevy::prelude::{
  App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, On, Plugin, Res, ResMut, in_state,
};
use mooplas_networking::prelude::{
  NetworkingErrorEvent, NetworkingMessagesPlugin, NetworkingResourcesPlugin, create_client, create_server,
  remove_all_resources,
};

/// Plugin that adds online multiplayer capabilities for native builds to the game.
pub struct NativeOnlinePlugin;

impl Plugin for NativeOnlinePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((NetworkingResourcesPlugin, NetworkingMessagesPlugin))
      .add_plugins((ClientPlugin, ServerPlugin))
      .add_systems(Update, handle_toggle_menu_message.run_if(in_state(AppState::Preparing)))
      .add_systems(
        Update,
        handle_connection_info_message
          .run_if(in_state(AppState::Preparing))
          .run_if(|network_role: Res<NetworkRole>| network_role.is_client()),
      )
      .add_observer(handle_netcode_transport_error_event);
    info!("Online multiplayer for native builds is enabled");
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
      NetworkRole::None => remove_all_resources(&mut commands),
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

#[allow(clippy::never_loop)]
fn handle_netcode_transport_error_event(error_event: On<NetworkingErrorEvent>) {
  let error = error_event.event();
  if matches!(
    error,
    &NetworkingErrorEvent::RenetDisconnect(_) | &NetworkingErrorEvent::NetcodeDisconnect(_)
  ) {
    return;
  }
  error!("Networking error occurred: [{}], panicking now...", error);
  panic!("{}", error);
}
