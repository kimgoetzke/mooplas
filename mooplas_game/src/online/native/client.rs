use crate::online::native::structs::NetworkTransformInterpolation;
use crate::online::native::utils;
use crate::prelude::constants::CLIENT_HAND_SHAKE_TIMEOUT_SECS;
use crate::prelude::{
  AppState, AvailablePlayerConfigs, ExitLobbyMessage, InputMessage, MenuName, NetworkRole, PlayerId,
  PlayerRegistrationMessage, RegisteredPlayers, Seed, SnakeHead, ToggleMenuMessage, UiNotification, WinnerInfo,
};
use bevy::app::Update;
use bevy::log::*;
use bevy::math::Quat;
use bevy::prelude::{
  App, Commands, Entity, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, On, Plugin, Query, Res, ResMut,
  State, Time, Transform, With, Without, in_state, resource_exists,
};
use bevy_renet::RenetClient;
use mooplas_networking::prelude::{
  ChannelType, ClientMessage, ClientRenetPlugin, ClientVisualiserPlugin, PendingClientHandshake,
  PlayerStateUpdateMessage, ServerEvent, encode_to_bytes, is_client_connected,
};
use std::time::Instant;

/// A plugin that adds client-side online multiplayer capabilities to the game. Only active when the application is
/// running in client mode (i.e. someone else is the server). Mutually exclusive with the
/// [`crate::online::native::ServerPlugin`].
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((ClientRenetPlugin, ClientVisualiserPlugin))
      .add_systems(
        Update,
        client_handshake_system.run_if(resource_exists::<PendingClientHandshake>),
      )
      .add_observer(handle_server_message_event)
      .add_systems(
        Update,
        (
          send_local_player_registration_system,
          process_and_send_local_exit_lobby_message_system,
        )
          .run_if(in_state(AppState::Registering))
          .run_if(is_client_connected),
      )
      .add_systems(
        Update,
        (
          send_local_input_messages,
          add_interpolation_component_system,
          apply_state_interpolation_system,
        )
          .run_if(in_state(AppState::Playing))
          .run_if(is_client_connected),
      );
  }
}

/// System that checks whether the client completed the handshake before the deadline.
/// If the handshake did not complete in time, it cleans up the client transport and
/// emits a UI error message.
pub fn client_handshake_system(
  mut commands: Commands,
  client: Res<RenetClient>,
  handshake: Option<Res<PendingClientHandshake>>,
  mut ui_message: MessageWriter<UiNotification>,
) {
  let handshake = match handshake {
    Some(h) => h,
    None => return,
  };

  if client.is_connected() {
    commands.remove_resource::<PendingClientHandshake>();
    info!("Client handshake completed");
    return;
  }

  let now = Instant::now();
  if now > handshake.deadline {
    let message = "Couldn't complete handshake with server - is there a typo in the connection string?".to_string();
    error!("Timed out after {}s: {}", CLIENT_HAND_SHAKE_TIMEOUT_SECS, message);
    ui_message.write(UiNotification::error(message));
    handshake.clean_up_after_failure(&mut commands);
  }
}

/// Processes any incoming [`DefaultChannel::ReliableOrdered`] server messages and acts on them, if required.
fn handle_server_message_event(
  server_message: On<ServerEvent>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  current_state: ResMut<State<AppState>>,
  mut next_state: ResMut<NextState<AppState>>,
  mut winner: ResMut<WinnerInfo>,
  mut seed: ResMut<Seed>,
  mut registration_message: MessageWriter<PlayerRegistrationMessage>,
  mut player_state_update_message: MessageWriter<PlayerStateUpdateMessage>,
  mut exit_lobby_message: MessageWriter<ExitLobbyMessage>,
) {
  match server_message.event() {
    ServerEvent::ClientConnected { client_id } => {
      info!("A client with ID [{:?}] connected", client_id);
    }
    ServerEvent::ClientDisconnected { client_id } => {
      info!("A client with ID [{:?}] disconnected", client_id);
    }
    ServerEvent::ClientInitialised { seed: server_seed, .. } => {
      seed.set(*server_seed);
    }
    ServerEvent::PlayerRegistered { player_id, .. } => {
      let player_id = PlayerId(*player_id);
      utils::register_player_locally(
        &mut registered_players,
        &available_configs,
        &mut registration_message,
        player_id,
      );
    }
    ServerEvent::PlayerUnregistered { player_id, .. } => {
      let player_id = PlayerId(*player_id);
      utils::unregister_player_locally(&mut registered_players, &mut registration_message, player_id);
    }
    ServerEvent::StateChanged { new_state, winner_info } => {
      if !current_state.is_manual_transition_allowed_to(&AppState::from(new_state)) {
        next_state.set(AppState::from(new_state));
      } else {
        debug!(
          "Ignoring state change to [{}] because [{:?}] is restricted...",
          new_state, *current_state
        );
      }
      if let Some(player_id) = winner_info {
        winner.set((*player_id).into());
      }
    }
    ServerEvent::ShutdownServer => {
      exit_lobby_message.write(ExitLobbyMessage::forced_by_server());
    }
    ServerEvent::UpdatePlayerStates { states } => {
      for (player_id, x, y, rotation_z) in states {
        player_state_update_message.write(PlayerStateUpdateMessage::new(*player_id, (*x, *y), *rotation_z));
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
    let client_message = ClientMessage::PlayerRegistration(player_registration_message.into());
    debug!("Sending: [{:?}]", client_message);
    let message = encode_to_bytes(&client_message).expect("Failed to serialise player registration message");
    client.send_message(ChannelType::ReliableOrdered, message);
  }
}

/// A system that handles local exit lobby messages by disconnecting from the server and returning to the main menu.
fn process_and_send_local_exit_lobby_message_system(
  mut messages: MessageReader<ExitLobbyMessage>,
  mut client: ResMut<RenetClient>,
  mut toggle_menu_message: MessageWriter<ToggleMenuMessage>,
  mut registered_players: ResMut<RegisteredPlayers>,
) {
  for message in messages.read() {
    debug!("Disconnecting from server (by force={})...", message.by_force);
    if !message.by_force {
      client.disconnect();
    }
    toggle_menu_message.write(ToggleMenuMessage::set(MenuName::MainMenu));
    registered_players.clear();
  }
}

/// A system that handles local input action messages for mutable players by sending them to the server in order to sync
/// the movements of the local player(s) with the server.
fn send_local_input_messages(
  mut messages: MessageReader<InputMessage>,
  registered_players: Res<RegisteredPlayers>,
  mut client: ResMut<RenetClient>,
) {
  for message in messages.read() {
    let player_id = match message {
      InputMessage::Action(player_id) => player_id,
      InputMessage::Move(player_id, _) => player_id,
    };
    if let Some(_) = registered_players
      .players
      .iter()
      .find(|player| player.id == *player_id && player.is_local())
    {
      if let Ok(input_message) = encode_to_bytes(&ClientMessage::Input(message.into())) {
        client.send_message(ChannelType::Unreliable, input_message);
      } else {
        warn!("Failed to serialise input action message: {:?}", message);
      }
    } else {
      error_once!(
        "Received input action message for player ID [{}], but no matching local player was found: {:?}",
        player_id,
        message
      );
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

/// Applies interpolation to remote players based on received state updates. Updates the interpolation target when new
/// states arrive, then interpolates towards them.
fn apply_state_interpolation_system(
  time: Res<Time>,
  mut player_state_messages: MessageReader<PlayerStateUpdateMessage>,
  mut snake_head_query: Query<(&mut Transform, &mut NetworkTransformInterpolation, &PlayerId), With<SnakeHead>>,
) {
  // Update targets based on incoming server messages
  for message in player_state_messages.read() {
    for (_, mut interpolation, player_id) in snake_head_query.iter_mut() {
      if player_id.0 == message.id {
        let target_position = bevy::math::Vec2::new(message.position.0, message.position.1);
        let target_rotation = Quat::from_rotation_z(message.rotation);
        interpolation.update_target(target_position, target_rotation);
      }
    }
  }

  // Interpolate all remote players towards their targets
  let delta = time.delta_secs();
  for (mut transform, interpolation, _) in snake_head_query.iter_mut() {
    let current_position = transform.translation.truncate();
    let target_position = interpolation.target_position;
    let new_position = current_position.lerp(target_position, interpolation.interpolation_speed * delta * 60.);
    transform.translation.x = new_position.x;
    transform.translation.y = new_position.y;
    let new_rotation = transform.rotation.slerp(
      interpolation.target_rotation,
      interpolation.interpolation_speed * delta * 60.,
    );
    transform.rotation = new_rotation;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::constants::RESOLUTION_WIDTH;
  use crate::prelude::{SharedMessagesPlugin, SharedResourcesPlugin};
  use bevy::math::Vec3;
  use bevy::prelude::*;
  use mooplas_networking::prelude::NetworkingMessagesPlugin;
  use std::time::Duration;

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Add shared messages and resources as they are required by the game loop systems
    app.add_plugins((SharedMessagesPlugin, SharedResourcesPlugin, NetworkingMessagesPlugin));
    app
  }

  fn advance_time_by(app: &mut App, duration: Duration) {
    let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
    info!("Time before manual update: {:?}", time.elapsed());
    time.advance_by(duration);
    app.update();
    let time = app.world_mut().get_resource_mut::<Time>().unwrap();
    info!("Time after manual update: {:?}", time.elapsed());
  }

  #[test]
  fn apply_state_interpolation_system_applies_interpolation_within_screen_bounds() {
    let mut app = setup();

    // Spawn an entity that the system will operate on
    let entity = app
      .world_mut()
      .spawn((
        Transform::from_translation(Vec3::new(100.0, 100.0, 0.0)),
        NetworkTransformInterpolation::new(2.),
        PlayerId(1),
        SnakeHead,
      ))
      .id();

    // Add the system to be tested and the message to be processed inside the system
    app.add_systems(Update, apply_state_interpolation_system);
    app
      .world_mut()
      .write_message(PlayerStateUpdateMessage::new(1, (110., 110.), 0.))
      .expect("Failed to write PlayerStateUpdateMessage message");
    app.update();

    // Advance the time a little
    advance_time_by(&mut app, Duration::from_millis(100));

    // Inspect the transform after interpolation and ensure it has moved towards the target
    let translation = app.world_mut().get::<Transform>(entity).unwrap().translation;
    assert!(translation.x > 100., "X position did not advance during interpolation");
    assert!(translation.y > 100., "Y position did not advance during interpolation");
  }

  #[test]
  fn apply_state_interpolation_system_handles_far_targets() {
    let mut app = setup();

    // Spawn an entity that the system will operate on at the right edge
    let entity = app
      .world_mut()
      .spawn((
        Transform::from_translation(Vec3::new(RESOLUTION_WIDTH as f32, 100., 0.)),
        NetworkTransformInterpolation::new(2.),
        PlayerId(1),
        SnakeHead,
      ))
      .id();

    // Add the system to be tested and the message to be processed inside the system - target is on the left side
    app.add_systems(Update, apply_state_interpolation_system);
    app
      .world_mut()
      .write_message(PlayerStateUpdateMessage::new(1, (0., 100.), 0.))
      .expect("Failed to write PlayerStateUpdateMessage message");
    app.update();

    // Advance the time a little
    advance_time_by(&mut app, Duration::from_millis(100));

    // After interpolation the X should have moved towards 0 (decreased)
    let translation = app.world_mut().get::<Transform>(entity).unwrap().translation;
    assert!(
      translation.y > 99.999 && translation.y < 100.001,
      "Y position should remain approximately 100"
    );
    assert!(
      translation.x < RESOLUTION_WIDTH as f32,
      "X position should move towards the target on the left"
    );
  }
}
