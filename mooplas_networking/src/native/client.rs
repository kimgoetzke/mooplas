use crate::native::{PendingClientHandshake, RenetClientVisualiser};
use crate::prelude::PROTOCOL_ID;
use bevy::log::*;
use bevy::prelude::{Commands, Plugin, Res};
use bevy_renet::netcode::{ClientAuthentication, NetcodeClientPlugin, NetcodeClientTransport};
use bevy_renet::renet::ConnectionConfig;
use bevy_renet::{RenetClient, RenetClientPlugin};
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

/// A Bevy plugin that adds the necessary Renet plugins. Required to run any client code on native.
pub struct ClientRenetPlugin;

impl Plugin for ClientRenetPlugin {
  fn build(&self, app: &mut bevy::prelude::App) {
    app.add_plugins((RenetClientPlugin, NetcodeClientPlugin));
  }
}

pub fn is_client_connected(client: Option<Res<RenetClient>>) -> bool {
  match client {
    Some(client) => client.is_connected(),
    None => false,
  }
}

pub fn create_client(commands: &mut Commands, server_address: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
  match create_new_renet_client_resources(server_address) {
    Ok((client, transport)) => {
      info!("Created client with connection to [{}]", server_address);
      commands.insert_resource(client);
      commands.insert_resource(transport);
      commands.insert_resource(PendingClientHandshake::new());
      commands.insert_resource(RenetClientVisualiser::default());
      Ok(())
    }
    Err(e) => Err(e),
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
