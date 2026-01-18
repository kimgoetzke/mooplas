#![cfg(feature = "online")]

mod client;
mod interface;
mod lib;
mod server;

use crate::online::client::ClientPlugin;
use crate::online::interface::InterfacePlugin;
use crate::online::lib::{NetworkingMessagesPlugin, NetworkingResourcesPlugin, PendingClientHandshake};
use crate::online::server::ServerPlugin;
use crate::prelude::constants::VISUALISER_DISPLAY_VALUES;
use crate::prelude::{AppState, MenuName, NetworkRole, ToggleMenuMessage, UiNotification};
use crate::shared::ConnectionInfoMessage;
use bevy::app::Update;
use bevy::log::*;
use bevy::prelude::{App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Res, ResMut, in_state};
use bevy_inspector_egui::egui::TextBuffer;
use bevy_renet::netcode::{
  ClientAuthentication, NetcodeClientTransport, NetcodeError, NetcodeServerTransport, NetcodeTransportError,
  ServerAuthentication, ServerConfig,
};
use bevy_renet::renet::{ConnectionConfig, RenetClient, RenetServer};
use renet_visualizer::{RenetClientVisualizer, RenetServerVisualizer};
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((ClientPlugin, ServerPlugin))
      .add_systems(Update, handle_toggle_menu_message.run_if(in_state(AppState::Preparing)))
      .add_systems(
        Update,
        handle_connection_info_message
          .run_if(in_state(AppState::Preparing))
          .run_if(|network_role: Res<NetworkRole>| network_role.is_client()),
      )
      .add_systems(Update, handle_netcode_transport_error_message)
      .add_plugins((InterfacePlugin, NetworkingResourcesPlugin, NetworkingMessagesPlugin));
    info!("Online multiplayer is enabled");
  }
}

const PROTOCOL_ID: u64 = 1000;
const DEFAULT_SERVER_PORT: u16 = 0;

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
        commands.remove_resource::<RenetServerVisualizer<VISUALISER_DISPLAY_VALUES>>();
        commands.remove_resource::<RenetClientVisualizer<VISUALISER_DISPLAY_VALUES>>();
      }
      NetworkRole::Server => {
        let port = DEFAULT_SERVER_PORT;
        match create_new_renet_server_resources(port) {
          Ok((server, transport)) => {
            debug!("Server started on {:?}", transport.addresses());
            connection_info_message.write(ConnectionInfoMessage {
              connection_string: transport.addresses()[0].to_string(),
            });
            commands.insert_resource(server);
            commands.insert_resource(transport);
            commands.insert_resource(RenetServerVisualizer::<VISUALISER_DISPLAY_VALUES>::default());
          }
          Err(e) => {
            error!("Failed to create server: {}", e);
            *network_role = NetworkRole::None;
          }
        }
      }
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
          commands.insert_resource(RenetClientVisualizer::<VISUALISER_DISPLAY_VALUES>::default());
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

fn create_new_renet_server_resources(
  port: u16,
) -> Result<(RenetServer, NetcodeServerTransport), Box<dyn std::error::Error>> {
  let bind_address: SocketAddr = SocketAddr::new(std::net::IpAddr::V6(Ipv6Addr::UNSPECIFIED), port);
  let socket = UdpSocket::bind(bind_address)?;
  let local_address = socket.local_addr()?;
  let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
  let public_address = get_public_ip_with_port(local_address.port()).unwrap_or(local_address);
  let server_config = ServerConfig {
    current_time,
    max_clients: 64,
    protocol_id: PROTOCOL_ID,
    public_addresses: vec![public_address],
    authentication: ServerAuthentication::Unsecure,
  };
  let transport = NetcodeServerTransport::new(server_config, socket)?;
  let server = RenetServer::new(ConnectionConfig::default());

  Ok((server, transport))
}

/// Attempts to determine the server's public IP address. Returns [`None`] if unable to determine (falls back to bind
/// address).
fn get_public_ip_with_port(port: u16) -> Option<SocketAddr> {
  let services = [
    "https://icanhazip.com",
    "https://api6.ipify.org",
    "https://ifconfig.me/ip",
  ];

  for service in &services {
    if let Ok(mut response) = ureq::get(service.as_str()).call() {
      if let Ok(response_body) = response.body_mut().read_to_string() {
        let ip_string = response_body.trim();
        if let Ok(ip) = ip_string.parse::<std::net::IpAddr>() {
          if ip.is_ipv6() {
            info!("Public IPv6 detected using [{}]: {}", service, ip);
            return Some(SocketAddr::new(ip, port));
          }
        }
        warn!("Invalid IP format received from service [{}]: {}", service, ip_string);
      } else {
        warn!("Failed to read response body from [{}]", service);
      }
    } else {
      warn!("Failed to get public IP from [{}]", service);
    }
  }

  error!("Failed to determine public IP address");
  None
}

#[allow(clippy::never_loop)]
fn handle_netcode_transport_error_message(mut messages: MessageReader<NetcodeTransportError>) {
  for error in messages.read() {
    if matches!(
      error,
      NetcodeTransportError::Renet(_) | NetcodeTransportError::Netcode(NetcodeError::Disconnected(_))
    ) {
      return;
    }
    error!("Netcode transport error occurred: [{}], panicking now...", error);
    panic!("{}", error);
  }
}
