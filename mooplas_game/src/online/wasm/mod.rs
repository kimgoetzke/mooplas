use crate::app_state::AppState;
use crate::prelude::{ConnectionInfoMessage, MenuName, ToggleMenuMessage};
use bevy::app::{App, Plugin, Update};
use bevy::log::*;
use bevy::prelude::{Commands, IntoScheduleConfigs, MessageReader, MessageWriter, ResMut, in_state};
use mooplas_networking::prelude::{ClientNetworkingActive, Lobby, NetworkRole, ServerNetworkingActive};
use mooplas_networking_matchbox::prelude::{
  ClientPlugin, HostPlugin, remove_all_matchbox_resources, start_signaling_server, start_socket,
};

// TODO: Implement a transport for WASM targets
/// Plugin that adds online multiplayer capabilities for WASM targets to the game.
pub struct WasmOnlinePlugin;

impl Plugin for WasmOnlinePlugin {
  fn build(&self, app: &mut App) {
    info!("Online multiplayer for WebAssembly builds is enabled");
    app
      .add_plugins((HostPlugin, ClientPlugin))
      .add_systems(Update, handle_toggle_menu_message.run_if(in_state(AppState::Preparing)));
  }
}

fn handle_toggle_menu_message(
  mut commands: Commands,
  mut messages: MessageReader<ToggleMenuMessage>,
  mut network_role: ResMut<NetworkRole>,
  mut lobby: ResMut<Lobby>,
  mut connection_info_message: MessageWriter<ConnectionInfoMessage>,
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
        start_socket(&mut commands);
        commands.insert_resource(ServerNetworkingActive);
      }
      NetworkRole::Client => {
        debug!("Waiting for connection info to create client...");
        start_socket(&mut commands);
        commands.insert_resource(ClientNetworkingActive);
      }
    }
    debug!("Network role set to [{:?}]", network_role);
  }
}
