use crate::app_states::AppState;
use crate::online::lib::{
  ClientMessage, InputSequence, NetworkClientId, SerialisableInputAction, ServerMessage, utils,
};
use crate::prelude::{PlayerId, PlayerRegistrationMessage, RegisteredPlayers, Seed};
use crate::shared::{AvailablePlayerConfigs, InputAction, NetworkAudience};
use bevy::app::Update;
use bevy::log::*;
use bevy::prelude::{App, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, Plugin, Res, ResMut, in_state};
use bevy_renet::netcode::NetcodeClientPlugin;
use bevy_renet::renet::{DefaultChannel, RenetClient};
use bevy_renet::{RenetClientPlugin, client_connected};

/// A plugin that adds client-side online multiplayer capabilities to the game. Only active when the application is
/// running in client mode (i.e. someone else is the server). Mutually exclusive with the
/// [`crate::online::ServerPlugin`].
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((RenetClientPlugin, NetcodeClientPlugin))
      .add_systems(Update, handle_reliable_server_messages_system.run_if(client_connected))
      .add_systems(
        Update,
        handle_local_player_registration_message
          .run_if(in_state(AppState::Registering))
          .run_if(client_connected),
      )
      .add_systems(
        Update,
        (
          handle_unreliable_messages_from_server_system,
          handle_local_input_action_messages,
        )
          .run_if(in_state(AppState::Playing))
          .run_if(client_connected),
      );
  }
}

/// Processes any incoming [`DefaultChannel::ReliableOrdered`] server messages and acts on them, if required.
fn handle_reliable_server_messages_system(
  mut client: ResMut<RenetClient>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut messages: MessageWriter<PlayerRegistrationMessage>,
  mut next_state: ResMut<NextState<AppState>>,
  mut seed: ResMut<Seed>,
  mut network_client_id: ResMut<NetworkClientId>,
) {
  while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
    let server_message = bincode::deserialize(&message).expect("Failed to deserialise server message");
    debug!("Received server message: {:?}", server_message);
    match server_message {
      ServerMessage::ClientConnected { client_id } => {
        info!("A client with ID [{}] connected", client_id);
      }
      ServerMessage::ClientDisconnected { client_id } => {
        info!("A client with ID [{}] disconnected", client_id);
      }
      ServerMessage::ClientInitialised {
        seed: server_seed,
        client_id,
      } => {
        seed.set(server_seed);
        network_client_id.set(client_id);
      }
      ServerMessage::PlayerRegistered { player_id, .. } => {
        let player_id = PlayerId(player_id);
        utils::register_player_locally(
          &mut registered_players,
          &available_configs,
          &mut messages,
          player_id,
          None,
        );
      }
      ServerMessage::PlayerUnregistered { player_id, .. } => {
        let player_id = PlayerId(player_id);
        utils::unregister_player_locally(&mut registered_players, &mut messages, player_id, None);
      }
      ServerMessage::StateChanged { new_state } => {
        debug!("Server changed state to [{}]", new_state);
        next_state.set(AppState::from(&new_state));
      }
    }
  }
}

/// A system that handles local player registration messages by sending them to the server.
fn handle_local_player_registration_message(
  mut messages: MessageReader<PlayerRegistrationMessage>,
  mut client: ResMut<RenetClient>,
) {
  for message in messages.read() {
    if utils::should_message_be_skipped(&message, NetworkAudience::Server) {
      continue;
    }
    let registration_message = bincode::serialize(&ClientMessage::PlayerRegistrationMessage(*message))
      .expect("Failed to serialise player registration message");
    debug!("Sending [{:?}] to server", registration_message);
    client.send_message(DefaultChannel::ReliableOrdered, registration_message);
  }
}

/// Processes any incoming [`DefaultChannel::Unreliable`] server messages and acts on them, if required.
fn handle_unreliable_messages_from_server_system(
  mut client: ResMut<RenetClient>,
  mut input_action_message: MessageWriter<InputAction>,
) {
  while let Some(message) = client.receive_message(DefaultChannel::Unreliable) {
    if let Ok(client_message) = bincode::deserialize(&message) {
      match client_message {
        ClientMessage::InputAction(_, action) => {
          input_action_message.write(action.into());
        }
        _ => {
          warn!(
            "Received unrecognised client message on [Unreliable] channel: {:?}",
            client_message
          );
        }
      }
    } else {
      warn!("Failed to deserialise [Unreliable] client message");
    }
  }
}

/// A system that handles local input action messages for mutable players by sending them to the server in order to sync
/// the movements of the local player(s) with the server.
fn handle_local_input_action_messages(
  mut messages: MessageReader<SerialisableInputAction>,
  registered_players: Res<RegisteredPlayers>,
  mut client: ResMut<RenetClient>,
  mut sequence: ResMut<InputSequence>,
) {
  for message in messages.read() {
    let player_id = match message {
      SerialisableInputAction::Action(player_id) => player_id,
      SerialisableInputAction::Move(player_id, _) => player_id,
    };
    if let Some(_) = registered_players
      .players
      .iter()
      .find(|player| player.id.0 == *player_id && player.mutable)
    {
      sequence.current = sequence.current.wrapping_add(1);
      if let Ok(input_message) = bincode::serialize(&ClientMessage::InputAction(sequence.current, *message)) {
        client.send_message(DefaultChannel::Unreliable, input_message);
      } else {
        warn!("Failed to serialise input action message: {:?}", message);
      }
    }
  }
}
