use crate::app_states::AppState;
use crate::online::lib::{ServerMessages, utils};
use crate::prelude::{PlayerId, PlayerRegistrationMessage, RegisteredPlayers};
use crate::shared::{AvailablePlayerConfigs, InputAction, NetworkAudience};
use bevy::app::Update;
use bevy::log::*;
use bevy::prelude::{App, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, Plugin, Res, ResMut, in_state};
use bevy_renet::netcode::NetcodeClientPlugin;
use bevy_renet::renet::{DefaultChannel, RenetClient};
use bevy_renet::{RenetClientPlugin, client_connected};

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((RenetClientPlugin, NetcodeClientPlugin))
      .add_systems(
        Update,
        handle_serialisable_input_message
          .run_if(in_state(AppState::Playing))
          .run_if(client_connected),
      )
      .add_systems(Update, client_sync_players_system.run_if(client_connected))
      .add_systems(
        Update,
        handle_local_player_registration_message
          .run_if(in_state(AppState::Registering))
          .run_if(client_connected),
      );
  }
}

/// Sending input action messages to the server. Syncs the movements of the local player with the server.
fn handle_serialisable_input_message(mut messages: MessageReader<InputAction>, mut client: ResMut<RenetClient>) {
  for message in messages.read() {
    let input_message = bincode::serialize(&message).unwrap();
    client.send_message(DefaultChannel::ReliableOrdered, input_message);
  }
}

fn handle_local_player_registration_message(
  mut messages: MessageReader<PlayerRegistrationMessage>,
  mut client: ResMut<RenetClient>,
) {
  for message in messages.read() {
    if utils::should_message_be_skipped(&message, NetworkAudience::Server) {
      continue;
    }
    debug!("Sending: {:?}...", message);
    let registration_message = bincode::serialize(&message).unwrap();
    client.send_message(DefaultChannel::ReliableOrdered, registration_message);
  }
}

fn client_sync_players_system(
  mut client: ResMut<RenetClient>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut messages: MessageWriter<PlayerRegistrationMessage>,
  mut next_state: ResMut<NextState<AppState>>,
) {
  while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
    let server_message = bincode::deserialize(&message).expect("Failed to deserialise server message");
    match server_message {
      ServerMessages::PlayerConnected { client_id } => {
        info!("A player with client ID [{}] connected", client_id);
      }
      ServerMessages::PlayerDisconnected { client_id } => {
        info!("A player with client ID [{}] disconnected", client_id);
      }
      ServerMessages::PlayerRegistered { client_id, player_id } => {
        info!(
          "[Player {}] with client ID [{}] attempts to register...",
          player_id, client_id
        );
        let player_id = PlayerId(player_id);
        utils::register_player_locally(
          &mut registered_players,
          &available_configs,
          &mut messages,
          player_id,
          None,
        );
      }
      ServerMessages::PlayerUnregistered { client_id, player_id } => {
        debug!(
          "[Player {}] with client ID [{}] attempts to unregister...",
          player_id, client_id
        );
        let player_id = PlayerId(player_id);
        utils::unregister_player_locally(&mut registered_players, &mut messages, player_id, None);
      }
      ServerMessages::StateChanged { new_state } => {
        debug!("Server changed state to [{}]", new_state);
        next_state.set(AppState::from(&new_state));
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
