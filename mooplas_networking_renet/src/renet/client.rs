use crate::prelude::ClientHandshakeOutcomeMessage;
use crate::renet::{CLIENT_HAND_SHAKE_TIMEOUT_SECS, PROTOCOL_ID, PendingClientHandshake, RenetClientVisualiser};
use bevy::app::Update;
use bevy::log::*;
use bevy::prelude::{
  Commands, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Res, ResMut, resource_exists,
};
use bevy_renet::netcode::{ClientAuthentication, NetcodeClientPlugin, NetcodeClientTransport};
use bevy_renet::renet::{ConnectionConfig, DefaultChannel};
use bevy_renet::{RenetClient, RenetClientPlugin};
use mooplas_networking::prelude::{
  ChannelType, ClientNetworkingActive, OutgoingClientMessage, ServerEvent, decode_from_bytes,
};
use std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use std::time::{Instant, SystemTime};

/// A Bevy plugin that adds the necessary Renet plugins. Required to run any client code on native.
pub struct ClientRenetPlugin;

impl Plugin for ClientRenetPlugin {
  fn build(&self, app: &mut bevy::prelude::App) {
    app
      .add_plugins((RenetClientPlugin, NetcodeClientPlugin))
      .add_systems(
        Update,
        client_handshake_system.run_if(resource_exists::<PendingClientHandshake>),
      )
      .add_systems(
        Update,
        receive_reliable_server_messages_system.run_if(is_client_connected),
      )
      .add_systems(Update, send_outgoing_client_messages_system.run_if(is_client_connected));
  }
}

/// Creates client resources with a specific server address and inserts them into the world. Returns an error if the
/// resources could not be created e.g. due to a socket binding failure.
///
/// Creates the temporary [`PendingClientHandshake`] resource which is used to track the progress of the client
/// handshake process.
///
/// If successful, you can use [`ClientNetworkingActive`] as a marker to check if the client is active and connected
/// going forward.
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

/// Returns true if the [`RenetClient`] resource exists i.e. the client is connected to a server.
fn is_client_connected(client: Option<Res<RenetClient>>) -> bool {
  match client {
    Some(client) => client.is_connected(),
    None => false,
  }
}

pub fn client_handshake_system(
  mut commands: Commands,
  handshake: Option<Res<PendingClientHandshake>>,
  client: Option<Res<RenetClient>>,
  mut client_handshake_outcome_message: MessageWriter<ClientHandshakeOutcomeMessage>,
) {
  let handshake = match handshake {
    Some(h) => h,
    None => return,
  };

  if is_client_connected(client) {
    commands.remove_resource::<PendingClientHandshake>();
    commands.insert_resource(ClientNetworkingActive::default());
    client_handshake_outcome_message.write(ClientHandshakeOutcomeMessage {
      has_succeeded: true,
      reason: Some("Successfully completed handshake with server".to_string()),
    });
    info!("Client handshake completed");
    return;
  }

  let now = Instant::now();
  if now > handshake.deadline {
    let message = "Couldn't complete handshake with server - is there a typo in the connection string?".to_string();
    error!("Timed out after {}s: {}", CLIENT_HAND_SHAKE_TIMEOUT_SECS, message);
    client_handshake_outcome_message.write(ClientHandshakeOutcomeMessage {
      has_succeeded: false,
      reason: Some(message),
    });
    handshake.clean_up_after_failure(&mut commands);
  }
}

/// A system that reads all messages from the server on all channels, deserialises them and triggers them as events for
/// an application to read and respond to.
fn receive_reliable_server_messages_system(mut client: ResMut<RenetClient>, mut commands: Commands) {
  while let Some(reliable_ordered_channel_message) = client.receive_message(DefaultChannel::ReliableOrdered) {
    let server_message: ServerEvent =
      decode_from_bytes(&reliable_ordered_channel_message).expect("Failed to deserialise server message");
    debug!(
      "Received [{:?}] server message: {:?}",
      ChannelType::ReliableOrdered,
      server_message
    );
    commands.trigger(server_message);
  }

  while let Some(unreliable_channel_message) = client.receive_message(DefaultChannel::Unreliable) {
    let server_message: ServerEvent =
      decode_from_bytes(&unreliable_channel_message).expect("Failed to deserialise server message");
    commands.trigger(server_message);
  }
}

/// A system that applies outgoing send/disconnect requests to the active [`RenetClient`].
fn send_outgoing_client_messages_system(
  mut outgoing_messages: MessageReader<OutgoingClientMessage>,
  mut client: ResMut<RenetClient>,
) {
  for outgoing_message in outgoing_messages.read() {
    match outgoing_message {
      OutgoingClientMessage::Send { channel, payload } => {
        client.send_message(*channel, payload.clone());
      }
      OutgoingClientMessage::Disconnect => {
        client.disconnect();
      }
    }
  }
}
