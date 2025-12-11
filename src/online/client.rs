use crate::online::lib::{
  ClientMessage, InputSequence, NetworkTransformInterpolation, PlayerStateUpdateMessage,
  SerialisableInputActionMessage, ServerMessage, utils,
};
use crate::prelude::{
  AppState, ExitLobbyMessage, MenuName, NetworkRole, PlayerId, PlayerRegistrationMessage, RegisteredPlayers, Seed,
  SnakeHead, WinnerInfo,
};
use crate::shared::{AvailablePlayerConfigs, ToggleMenuMessage};
use bevy::app::Update;
use bevy::log::*;
use bevy::math::Quat;
use bevy::prelude::{
  App, Commands, Entity, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, Plugin, Query, Res, ResMut,
  State, Time, Transform, With, Without, in_state,
};
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
      .add_systems(Update, receive_reliable_server_messages_system.run_if(client_connected))
      .add_systems(
        Update,
        (
          send_local_player_registration_system,
          send_local_exit_lobby_message_system,
        )
          .run_if(in_state(AppState::Registering))
          .run_if(client_connected),
      )
      .add_systems(
        Update,
        (
          receive_unreliable_server_messages_system,
          send_local_input_messages,
          add_interpolation_component_system,
          apply_state_interpolation_system,
        )
          .run_if(in_state(AppState::Playing))
          .run_if(client_connected),
      );
  }
}

/// Processes any incoming [`DefaultChannel::ReliableOrdered`] server messages and acts on them, if required.
fn receive_reliable_server_messages_system(
  mut client: ResMut<RenetClient>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut messages: MessageWriter<PlayerRegistrationMessage>,
  current_state: ResMut<State<AppState>>,
  mut next_state: ResMut<NextState<AppState>>,
  mut winner: ResMut<WinnerInfo>,
  mut seed: ResMut<Seed>,
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
      ServerMessage::ClientInitialised { seed: server_seed, .. } => {
        seed.set(server_seed);
      }
      ServerMessage::PlayerRegistered { player_id, .. } => {
        let player_id = PlayerId(player_id);
        utils::register_player_locally(&mut registered_players, &available_configs, &mut messages, player_id);
      }
      ServerMessage::PlayerUnregistered { player_id, .. } => {
        let player_id = PlayerId(player_id);
        utils::unregister_player_locally(&mut registered_players, &mut messages, player_id);
      }
      ServerMessage::StateChanged { new_state, winner_info } => {
        if !current_state.is_restricted() {
          next_state.set(AppState::from(&new_state));
        } else {
          debug!(
            "Ignoring state change to [{}] because [{:?}] is restricted...",
            new_state, *current_state
          );
        }
        if let Some(player_id) = winner_info {
          winner.set(player_id);
        }
      }
      _ => {
        warn!(
          "Received unexpected message on [ReliableOrdered] channel: {:?}",
          server_message
        );
      }
    }
  }
}

/// A system that handles local player registration messages by sending them to the server.
fn send_local_player_registration_system(
  mut messages: MessageReader<PlayerRegistrationMessage>,
  mut client: ResMut<RenetClient>,
) {
  for player_registration_message in messages.read() {
    if utils::should_message_be_skipped(&player_registration_message, NetworkRole::Server) {
      continue;
    }
    let client_message = ClientMessage::PlayerRegistration(*player_registration_message);
    debug!("Sending: [{:?}]", client_message);
    let message = bincode::serialize(&client_message).expect("Failed to serialise player registration message");
    client.send_message(DefaultChannel::ReliableOrdered, message);
  }
}

/// A system that handles local exit lobby messages by disconnecting from the server and returning to the main menu.
fn send_local_exit_lobby_message_system(
  mut messages: MessageReader<ExitLobbyMessage>,
  mut client: ResMut<RenetClient>,
  mut toggle_menu_message: MessageWriter<ToggleMenuMessage>,
  mut registered_players: ResMut<RegisteredPlayers>,
) {
  for _ in messages.read() {
    debug!("Disconnecting from server...");
    client.disconnect();
    toggle_menu_message.write(ToggleMenuMessage::set(MenuName::MainMenu));
    registered_players.clear();
  }
}

/// A system that handles local input action messages for mutable players by sending them to the server in order to sync
/// the movements of the local player(s) with the server.
fn send_local_input_messages(
  mut messages: MessageReader<SerialisableInputActionMessage>,
  registered_players: Res<RegisteredPlayers>,
  mut client: ResMut<RenetClient>,
  mut sequence: ResMut<InputSequence>,
) {
  for message in messages.read() {
    let player_id = match message {
      SerialisableInputActionMessage::Action(player_id) => player_id,
      SerialisableInputActionMessage::Move(player_id, _) => player_id,
    };
    if let Some(_) = registered_players
      .players
      .iter()
      .find(|player| player.id.0 == *player_id && player.is_local())
    {
      if let Ok(input_message) = bincode::serialize(&ClientMessage::Input(sequence.next(), *message)) {
        client.send_message(DefaultChannel::Unreliable, input_message);
      } else {
        warn!("Failed to serialise input action message: {:?}", message);
      }
    }
  }
}

/// Processes any incoming [`DefaultChannel::Unreliable`] server messages and acts on them, if required.
fn receive_unreliable_server_messages_system(
  mut client: ResMut<RenetClient>,
  mut player_state_update_message: MessageWriter<PlayerStateUpdateMessage>,
) {
  while let Some(message) = client.receive_message(DefaultChannel::Unreliable) {
    if let Ok(server_message) = bincode::deserialize(&message) {
      match server_message {
        ServerMessage::UpdatePlayerStates { states } => {
          for (player_id, x, y, rotation_z) in states {
            player_state_update_message.write(PlayerStateUpdateMessage::new(player_id, (x, y), rotation_z));
          }
        }
        _ => {
          warn!(
            "Received unexpected server message on [Unreliable] channel: {:?}",
            server_message
          );
        }
      }
    } else {
      warn!("Failed to deserialise [Unreliable] server message");
    }
  }
}

// TODO: Consider if I should really interpolate local players too
/// Adds a [`NetworkTransformInterpolation`] component to every snake heads.
fn add_interpolation_component_system(
  mut commands: Commands,
  snake_head_query: Query<Entity, (With<SnakeHead>, Without<NetworkTransformInterpolation>)>,
) {
  for entity in snake_head_query.iter() {
    commands.entity(entity).insert(NetworkTransformInterpolation::new(0.3));
  }
}

// TODO: Deal with snake tails that are wrapped around the screen somewhere
/// Applies interpolation to remote players based on received state updates. Updates the interpolation target when new
/// states arrive, then interpolates towards them.
fn apply_state_interpolation_system(
  time: Res<Time>,
  mut player_state_messages: MessageReader<PlayerStateUpdateMessage>,
  mut snake_head_query: Query<(&mut Transform, &mut NetworkTransformInterpolation, &PlayerId), With<SnakeHead>>,
) {
  // Update targets based on incoming messages
  for state_update in player_state_messages.read() {
    for (_, mut interpolation, player_id) in snake_head_query.iter_mut() {
      if player_id.0 == state_update.id {
        let target_position = bevy::math::Vec2::new(state_update.position.0, state_update.position.1);
        let target_rotation = Quat::from_rotation_z(state_update.rotation);
        interpolation.update_target(target_position, target_rotation);
      }
    }
  }

  // Interpolate all remote players towards their targets
  let delta = time.delta_secs();
  for (mut transform, interpolation, _) in snake_head_query.iter_mut() {
    let current_position = transform.translation.truncate();
    let target_position = interpolation.target_position;
    let new_position = current_position.lerp(target_position, interpolation.interpolation_speed * delta * 60.0);
    transform.translation.x = new_position.x;
    transform.translation.y = new_position.y;
    let new_rotation = transform.rotation.slerp(
      interpolation.target_rotation,
      interpolation.interpolation_speed * delta * 60.0,
    );
    transform.rotation = new_rotation;
  }
}
