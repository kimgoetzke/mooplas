#![cfg(feature = "online")]

mod interface;
mod lib;

use crate::app_states::AppState;
use crate::online::interface::InterfacePlugin;
use crate::online::lib::{
  Lobby, NetworkRole, NetworkingResourcesPlugin, OnlinePlayer, SerialisableInputAction, ServerMessages,
};
use crate::prelude::{InputAction, MenuName, PlayerId, PlayerRegistrationMessage, ToggleMenuMessage};
use crate::shared::RegisteredPlayers;
use crate::shared::constants::MOVEMENT_SPEED;
use bevy::app::Update;
use bevy::log::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::{
  App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, Query, Res, ResMut, Time, Transform,
  in_state, resource_exists,
};
use bevy_renet::netcode::{
  ClientAuthentication, NetcodeClientPlugin, NetcodeClientTransport, NetcodeServerPlugin, NetcodeServerTransport,
  NetcodeTransportError, ServerAuthentication, ServerConfig,
};
use bevy_renet::renet::{ClientId, ConnectionConfig, DefaultChannel, RenetClient, RenetServer, ServerEvent};
use bevy_renet::{RenetClientPlugin, RenetServerPlugin, client_connected};
use std::net::UdpSocket;
use std::time::SystemTime;

/// Plugin that adds online multiplayer capabilities to the game.
pub struct OnlinePlugin;

impl Plugin for OnlinePlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((RenetServerPlugin, NetcodeServerPlugin))
      .add_plugins((RenetClientPlugin, NetcodeClientPlugin))
      .add_systems(Update, handle_toggle_menu_message.run_if(in_state(AppState::Preparing)))
      .add_systems(
        Update,
        (handle_serialisable_input_message, client_sync_players_system)
          .chain()
          .run_if(client_connected),
      )
      .add_systems(
        Update,
        (
          server_update_system,
          server_sync_players_system,
          // server_move_players_system,
        )
          .run_if(resource_exists::<RenetServer>),
      )
      .add_systems(Update, panic_on_error_system)
      .add_plugins((InterfacePlugin, NetworkingResourcesPlugin));
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

fn server_sync_players_system(mut server: ResMut<RenetServer>, query: Query<(&Transform, &OnlinePlayer)>) {
  let mut lobby: HashMap<ClientId, [f32; 3]> = HashMap::new();
  for (transform, player) in query.iter() {
    lobby.insert(player.id, transform.translation.into());
  }

  let sync_message = bincode::serialize(&lobby).expect("Failed to serialize sync message");
  server.broadcast_message(DefaultChannel::Unreliable, sync_message);
}

fn server_update_system(
  mut server_events: MessageReader<ServerEvent>,
  mut commands: Commands,
  mut lobby: ResMut<Lobby>,
  mut server: ResMut<RenetServer>,
) {
  debug_once!("Server update system running");
  for event in server_events.read() {
    // match event {
    //   ServerEvent::ClientConnected { client_id } => {
    //     println!("Player {} connected.", client_id);
    //     // Spawn player cube
    //     let player_entity = commands
    //       .spawn((
    //         Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(1.0)))),
    //         MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
    //         Transform::from_xyz(0.0, 0.5, 0.0),
    //       ))
    //       .insert(PlayerInput::default())
    //       .insert(Player { id: *client_id })
    //       .id();
    //
    //     // We could send an InitState with all the players id and positions for the client
    //     // but this is easier to do.
    //     for &player_id in lobby.players.keys() {
    //       let message = bincode::serialize(&ServerMessages::PlayerConnected { id: player_id }).unwrap();
    //       server.send_message(*client_id, DefaultChannel::ReliableOrdered, message);
    //     }
    //
    //     lobby.players.insert(*client_id, player_entity);
    //
    //     let message = bincode::serialize(&ServerMessages::PlayerConnected { id: *client_id }).unwrap();
    //     server.broadcast_message(DefaultChannel::ReliableOrdered, message);
    //   }
    //   ServerEvent::ClientDisconnected { client_id, reason } => {
    //     println!("Player {} disconnected: {}", client_id, reason);
    //     if let Some(player_entity) = lobby.players.remove(client_id) {
    //       commands.entity(player_entity).despawn();
    //     }
    //
    //     let message = bincode::serialize(&ServerMessages::PlayerDisconnected { id: *client_id }).unwrap();
    //     server.broadcast_message(DefaultChannel::ReliableOrdered, message);
    //   }
    // }
  }

  // for client_id in server.clients_id() {
  //   while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
  //     let player_input: PlayerInput = bincode::deserialize(&message).unwrap();
  //     if let Some(player_entity) = lobby.players.get(&client_id) {
  //       commands.entity(*player_entity).insert(player_input);
  //     }
  //   }
  // }
}

// fn server_move_players_system(mut query: Query<(&mut Transform, &PlayerInput)>, time: Res<Time>) {
//   for (mut transform, input) in query.iter_mut() {
//     let x = (input.right as i8 - input.left as i8) as f32;
//     let y = (input.down as i8 - input.up as i8) as f32;
//     transform.translation.x += x * MOVEMENT_SPEED * time.delta().as_secs_f32();
//     transform.translation.y += y * MOVEMENT_SPEED * time.delta().as_secs_f32();
//   }
// }

fn handle_serialisable_input_message(
  mut messages: MessageReader<SerialisableInputAction>,
  mut client: ResMut<RenetClient>,
) {
  for message in messages.read() {
    debug!("Sending input action: {:?}", message);
    let input_message = bincode::serialize(&message).unwrap();
    client.send_message(DefaultChannel::ReliableOrdered, input_message);
  }
}

fn client_sync_players_system(
  mut commands: Commands,
  mut client: ResMut<RenetClient>,
  mut messages: MessageWriter<PlayerRegistrationMessage>,
  registered_players: ResMut<RegisteredPlayers>,
) {
  debug_once!("Client sync players system running");
  while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
    let server_message = bincode::deserialize(&message).expect("Failed to deserialize server message");
    match server_message {
      ServerMessages::PlayerConnected { client_id, player_id } => {
        info!("[Player {}] with client ID [{}] connected", player_id, client_id);
        messages.write(PlayerRegistrationMessage {
          player_id: PlayerId(player_id),
          has_registered: true,
          is_anyone_registered: true,
        });
      }
      ServerMessages::PlayerDisconnected { client_id, player_id } => {
        info!("[Player {}] with client ID [{}] connected", player_id, client_id);
        messages.write(PlayerRegistrationMessage {
          player_id: PlayerId(player_id),
          has_registered: false,
          is_anyone_registered: registered_players.players.len() != 0,
        });
      }
    }
  }

  // while let Some(message) = client.receive_message(DefaultChannel::Unreliable) {
  //   let players: HashMap<ClientId, [f32; 3]> = bincode::deserialize(&message).unwrap();
  //   for (player_id, translation) in players.iter() {
  //     if let Some(player_entity) = lobby.players.get(player_id) {
  //       let transform = Transform {
  //         translation: (*translation).into(),
  //         ..Default::default()
  //       };
  //       commands.entity(*player_entity).insert(transform);
  //     }
  //   }
  // }
}

#[allow(clippy::never_loop)]
fn panic_on_error_system(mut messages: MessageReader<NetcodeTransportError>) {
  for error in messages.read() {
    error!("Netcode transport error occurred, panicking now...");
    panic!("{}", error);
  }
}
