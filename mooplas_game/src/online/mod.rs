#![cfg(feature = "online")]

mod client;
mod interface;
mod lib;
mod server;

use crate::online::client::ClientPlugin;
use crate::online::interface::InterfacePlugin;
use crate::online::lib::NetworkingMessagesPlugin;
use crate::online::server::ServerPlugin;
use crate::prelude::{AppState, MenuName, NetworkRole, ToggleMenuMessage, UiNotification};
use crate::shared::ConnectionInfoMessage;
use bevy::app::Update;
use bevy::log::*;
use bevy::prelude::{
  App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, On, Plugin, Res, ResMut, in_state,
};
use bevy_renet::netcode::{
  ClientAuthentication, NetcodeClientTransport, NetcodeError, NetcodeErrorEvent, NetcodeServerTransport,
  NetcodeTransportError,
};
use bevy_renet::renet::ConnectionConfig;
use bevy_renet::{RenetClient, RenetServer};
use mooplas_networking::prelude::{
  NetworkingResourcesPlugin, PendingClientHandshake, RenetClientVisualiser, RenetServerVisualiser, create_server,
};
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((NetworkingResourcesPlugin))
      .add_plugins((ClientPlugin, ServerPlugin))
      .add_systems(Update, handle_toggle_menu_message.run_if(in_state(AppState::Preparing)))
      .add_systems(
        Update,
        handle_connection_info_message
          .run_if(in_state(AppState::Preparing))
          .run_if(|network_role: Res<NetworkRole>| network_role.is_client()),
      )
      .add_observer(handle_netcode_transport_error_event)
      .add_plugins((InterfacePlugin, NetworkingMessagesPlugin));
    info!("Online multiplayer is enabled");
  }
}

const PROTOCOL_ID: u64 = 1000;

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
      NetworkRole::None => {
        commands.remove_resource::<RenetServer>();
        commands.remove_resource::<NetcodeServerTransport>();
        commands.remove_resource::<RenetClient>();
        commands.remove_resource::<NetcodeClientTransport>();
        commands.remove_resource::<RenetServerVisualiser>();
        commands.remove_resource::<RenetClientVisualiser>();
      }
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
      match create_new_renet_client_resources(server_address) {
        Ok((client, transport)) => {
          info!("Created client with connection to [{}]", server_address);
          commands.insert_resource(client);
          commands.insert_resource(transport);
          commands.insert_resource(PendingClientHandshake::new());
          commands.insert_resource(RenetClientVisualiser::default());
        }
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

// TODO: Add secure authentication
/// Creates client resources with a specific server address
fn create_new_renet_client_resources(
  server_address: SocketAddr,
) -> Result<(RenetClient, NetcodeClientTransport), Box<dyn std::error::Error>> {
  let bind_address = SocketAddr::new(std::net::IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0);
  let socket = UdpSocket::bind(bind_address)?;
  let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
  let client_id = current_time.as_millis() as u64;
  let authentication = ClientAuthentication::Unsecure {
    client_id,
    protocol_id: PROTOCOL_ID,
    server_addr: server_address,
    user_data: None,
  };
  let transport = NetcodeClientTransport::new(current_time, authentication, socket)?;
  let client = RenetClient::new(ConnectionConfig::default());

  Ok((client, transport))
}

#[allow(clippy::never_loop)]
fn handle_netcode_transport_error_event(error_event: On<NetcodeErrorEvent>) {
  if matches!(
    **error_event,
    NetcodeTransportError::Renet(_) | NetcodeTransportError::Netcode(NetcodeError::Disconnected(_))
  ) {
    return;
  }
  error!(
    "Netcode transport error occurred: [{}], panicking now...",
    **error_event
  );
  panic!("{}", **error_event);
}
