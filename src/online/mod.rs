#![cfg(feature = "online")]

mod client;
mod interface;
mod lib;
mod server;

use crate::app_states::AppState;
use crate::online::client::ClientPlugin;
use crate::online::interface::InterfacePlugin;
use crate::online::lib::{NetworkingMessagesPlugin, NetworkingResourcesPlugin};
use crate::online::server::ServerPlugin;
use crate::prelude::{MenuName, NetworkRole, ToggleMenuMessage};
use bevy::app::Update;
use bevy::log::*;
use bevy::prelude::{App, Commands, IntoScheduleConfigs, MessageReader, Plugin, ResMut, in_state};
use bevy_renet::netcode::{
  ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, NetcodeTransportError, ServerAuthentication,
  ServerConfig,
};
use bevy_renet::renet::{ConnectionConfig, RenetClient, RenetServer};
use std::net::UdpSocket;
use std::time::SystemTime;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((ClientPlugin, ServerPlugin))
      .add_systems(Update, handle_toggle_menu_message.run_if(in_state(AppState::Preparing)))
      .add_systems(Update, panic_on_error_system)
      .add_plugins((InterfacePlugin, NetworkingResourcesPlugin, NetworkingMessagesPlugin));
    info!("Online multiplayer is enabled");
  }
}

const PROTOCOL_ID: u64 = 1000;

fn handle_toggle_menu_message(
  mut commands: Commands,
  mut messages: MessageReader<ToggleMenuMessage>,
  mut network_role: ResMut<NetworkRole>,
) {
  for message in messages.read() {
    match message.active {
      MenuName::MainMenu => *network_role = NetworkRole::None,
      MenuName::HostGameMenu => *network_role = NetworkRole::Server,
      MenuName::JoinGameMenu => *network_role = NetworkRole::Client,
      _ => {}
    }
    match *network_role {
      NetworkRole::None => {
        commands.remove_resource::<RenetServer>();
        commands.remove_resource::<NetcodeServerTransport>();
        commands.remove_resource::<RenetClient>();
        commands.remove_resource::<NetcodeClientTransport>();
      }
      NetworkRole::Server => {
        let (server, transport) = create_new_renet_server_resources();
        commands.insert_resource(server);
        commands.insert_resource(transport);
      }
      NetworkRole::Client => {
        let (client, transport) = create_new_renet_client_resources();
        commands.insert_resource(client);
        commands.insert_resource(transport);
      }
    }
    debug!("Network role set to [{:?}]", network_role);
  }
}

fn create_new_renet_client_resources() -> (RenetClient, NetcodeClientTransport) {
  let server_addr = "127.0.0.1:5000".parse().unwrap();
  let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
  let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
  let client_id = current_time.as_millis() as u64;
  let authentication = ClientAuthentication::Unsecure {
    client_id,
    protocol_id: PROTOCOL_ID,
    server_addr,
    user_data: None,
  };

  let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();
  let client = RenetClient::new(ConnectionConfig::default());

  (client, transport)
}

fn create_new_renet_server_resources() -> (RenetServer, NetcodeServerTransport) {
  let public_addr = "127.0.0.1:5000".parse().unwrap();
  let socket = UdpSocket::bind(public_addr).unwrap();
  let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
  let server_config = ServerConfig {
    current_time,
    max_clients: 64,
    protocol_id: PROTOCOL_ID,
    public_addresses: vec![public_addr],
    authentication: ServerAuthentication::Unsecure,
  };

  let transport = NetcodeServerTransport::new(server_config, socket).unwrap();
  let server = RenetServer::new(ConnectionConfig::default());

  (server, transport)
}

#[allow(clippy::never_loop)]
fn panic_on_error_system(mut messages: MessageReader<NetcodeTransportError>) {
  for error in messages.read() {
    error!("Netcode transport error occurred, panicking now...");
    panic!("{}", error);
  }
}
