use crate::prelude::{MooplasServerEvent, PROTOCOL_ID, RenetServerVisualiser, ServerNetworkingActive};
use bevy::app::Plugin;
use bevy::log::{debug, error, info, warn};
use bevy::prelude::{App, Commands, On};
use bevy_renet::netcode::{NetcodeServerPlugin, NetcodeServerTransport, ServerAuthentication, ServerConfig};
use bevy_renet::renet::{ConnectionConfig, ServerEvent};
use bevy_renet::{RenetServer, RenetServerEvent, RenetServerPlugin};
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

/// A Bevy plugin that adds the necessary Renet plugins. Required to run any server code on native.
pub struct ServerRenetPlugin;

impl Plugin for ServerRenetPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((RenetServerPlugin, NetcodeServerPlugin))
      .add_observer(receive_renet_server_events);
  }
}

const DEFAULT_SERVER_PORT: u16 = 0;

pub fn create_server(commands: &mut Commands) -> Result<String, Box<dyn std::error::Error>> {
  let port = DEFAULT_SERVER_PORT;
  match create_new_renet_server_resources(port) {
    Ok((server, transport)) => {
      debug!("Server started on {:?}", transport.addresses());
      let connection_string = transport.addresses()[0].to_string();
      commands.insert_resource(server);
      commands.insert_resource(transport);
      commands.insert_resource(RenetServerVisualiser::default());
      commands.insert_resource(ServerNetworkingActive::default());
      Ok(connection_string)
    }
    Err(e) => Err(e),
  }
}

pub fn create_new_renet_server_resources(
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

/// Receives events from the [`RenetServer`] and triggers corresponding [`MooplasServerEvent`] for the game to react to.
fn receive_renet_server_events(server_event: On<RenetServerEvent>, mut commands: Commands) {
  let mooplas_server_event = match **server_event {
    ServerEvent::ClientConnected { client_id } => MooplasServerEvent::ClientConnected(client_id.into()),
    ServerEvent::ClientDisconnected { client_id, reason } => {
      MooplasServerEvent::ClientDisconnected(client_id.into(), reason.to_string())
    }
  };
  commands.trigger(mooplas_server_event);
}

/// Attempts to determine the server's public IP address. Returns [`None`] if unable to determine (falls back to bind
/// address).
fn get_public_ip_with_port(port: u16) -> Option<SocketAddr> {
  let services = [
    "https://icanhazip.com",
    "https://api6.ipify.org",
    "https://ifconfig.me/ip",
  ];

  for service in services {
    if let Ok(mut response) = ureq::get(service).call() {
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
