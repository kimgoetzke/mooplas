#![cfg(feature = "online")]

mod client;
mod interface;
mod lib;
mod server;

use crate::online::client::ClientPlugin;
use crate::online::interface::InterfacePlugin;
use crate::online::lib::{NetworkingMessagesPlugin, NetworkingResourcesPlugin};
use crate::online::server::ServerPlugin;
use crate::prelude::{AppState, MenuName, NetworkRole, ToggleMenuMessage};
use crate::shared::ConnectionInfoMessage;
use bevy::app::Update;
use bevy::log::*;
use bevy::prelude::{App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, ResMut, in_state};
use bevy_inspector_egui::egui::TextBuffer;
use bevy_renet::netcode::{
  ClientAuthentication, NetcodeClientTransport, NetcodeError, NetcodeServerTransport, NetcodeTransportError,
  ServerAuthentication, ServerConfig,
};
use bevy_renet::renet::{ConnectionConfig, RenetClient, RenetServer};
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((ClientPlugin, ServerPlugin))
      .add_systems(Update, handle_toggle_menu_message.run_if(in_state(AppState::Preparing)))
      .add_systems(Update, handle_netcode_transport_errors)
      .add_plugins((InterfacePlugin, NetworkingResourcesPlugin, NetworkingMessagesPlugin));
    info!("Online multiplayer is enabled");
  }
}

const PROTOCOL_ID: u64 = 1000;
const DEFAULT_SERVER_PORT: u16 = 0;

// TODO: Implement host/client menus and waiting states
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
      }
      NetworkRole::Server => {
        let port = DEFAULT_SERVER_PORT;
        match create_new_renet_server_resources(port) {
          Ok((server, transport)) => {
            info!("Server started on {:?}", transport.addresses());
            connection_info_message.write(ConnectionInfoMessage {
              server_address: transport.addresses()[0].ip().to_string(),
              server_port: transport.addresses()[0].port(),
            });
            commands.insert_resource(server);
            commands.insert_resource(transport);
          }
          Err(e) => {
            error!("Failed to create server: {}", e);
            *network_role = NetworkRole::None;
          }
        }
      }
      NetworkRole::Client => {
        // if let Some(server_address) = connection_info.server_address {
        //   match create_new_renet_client_resources(server_address) {
        //     Ok((client, transport)) => {
        //       info!("Created client to connect to [{}]", server_address);
        //       commands.insert_resource(client);
        //       commands.insert_resource(transport);
        //     }
        //     Err(e) => {
        //       error!("Failed to create client: {}", e);
        //       *network_role = NetworkRole::None;
        //     }
        //   }
        // } else {
        //   error!("No server address provided for client connection");
        //   *network_role = NetworkRole::None;
        // }
      }
    }
    debug!("Network role set to [{:?}]", network_role);
  }
}

// TODO: Add secure authentication
/// Creates client resources with a specific server address
fn create_new_renet_client_resources(
  server_address: SocketAddr,
) -> Result<(RenetClient, NetcodeClientTransport), Box<dyn std::error::Error>> {
  let socket = UdpSocket::bind(Ipv6Addr::UNSPECIFIED.to_string())?;
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
  let bind_address: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
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
          info!("Public IP detected (using response from [{}]: {}", service, ip);
          return Some(SocketAddr::new(ip, port));
        } else {
          warn!("Invalid IP format received from service [{}]: {}", service, ip_string);
        }
      } else {
        warn!("Failed to read response body from [{}]", service);
      }
    } else {
      warn!("Failed to get public IP from [{}]", service);
    }
  }

  warn!("Could not determine public IP, will use local network IP");
  if let Ok(socket) = UdpSocket::bind(Ipv6Addr::UNSPECIFIED.to_string()) {
    if socket.connect("8.8.8.8:80").is_ok() {
      if let Ok(local_address) = socket.local_addr() {
        let mut address = local_address;
        address.set_port(port);
        return Some(address);
      }
    }
  }

  None
}

#[allow(clippy::never_loop)]
fn handle_netcode_transport_errors(mut messages: MessageReader<NetcodeTransportError>) {
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
