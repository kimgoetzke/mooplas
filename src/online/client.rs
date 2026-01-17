use crate::online::lib::{
  ClientMessage, NetworkTransformInterpolation, PendingClientHandshake, PlayerStateUpdateMessage,
  SerialisableInputActionMessage, ServerMessage, decode_from_bytes, encode_to_bytes, utils,
};
use crate::prelude::constants::{
  CLIENT_HAND_SHAKE_TIMEOUT_SECS, SHOW_VISUALISERS_BY_DEFAULT, VISUALISER_DISPLAY_VALUES,
};
use crate::prelude::{
  AppState, ExitLobbyMessage, MenuName, NetworkRole, PlayerId, PlayerRegistrationMessage, RegisteredPlayers, Seed,
  SnakeHead, UiNotification, WinnerInfo,
};
use crate::shared::constants::{RESOLUTION_HEIGHT, RESOLUTION_WIDTH, WRAPAROUND_MARGIN};
use crate::shared::{AvailablePlayerConfigs, ToggleMenuMessage};
use bevy::app::Update;
use bevy::input::common_conditions::input_toggle_active;
use bevy::log::*;
use bevy::math::Quat;
use bevy::prelude::{
  App, Commands, Entity, IntoScheduleConfigs, KeyCode, MessageReader, MessageWriter, NextState, Plugin, Query, Res,
  ResMut, State, Time, Transform, With, Without, in_state, resource_exists,
};
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_renet::netcode::{NetcodeClientPlugin, NetcodeClientTransport};
use bevy_renet::renet::{DefaultChannel, RenetClient};
use bevy_renet::{RenetClientPlugin, client_connected};
use renet_visualizer::RenetClientVisualizer;
use std::time::Instant;

/// A plugin that adds client-side online multiplayer capabilities to the game. Only active when the application is
/// running in client mode (i.e. someone else is the server). Mutually exclusive with the
/// [`crate::online::ServerPlugin`].
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((RenetClientPlugin, NetcodeClientPlugin))
      .add_systems(
        Update,
        update_client_visualiser_system
          .run_if(resource_exists::<RenetClientVisualizer<VISUALISER_DISPLAY_VALUES>>)
          .run_if(input_toggle_active(SHOW_VISUALISERS_BY_DEFAULT, KeyCode::F2)),
      )
      .add_systems(
        Update,
        client_handshake_system.run_if(resource_exists::<PendingClientHandshake>),
      )
      .add_systems(Update, receive_reliable_server_messages_system.run_if(client_connected))
      .add_systems(
        Update,
        (
          send_local_player_registration_system,
          process_and_send_local_exit_lobby_message_system,
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

/// System that updates and displays the Renet client visualiser when toggled by the user.
fn update_client_visualiser_system(
  mut egui_contexts: EguiContexts,
  mut visualizer: ResMut<RenetClientVisualizer<200>>,
  client: Res<RenetClient>,
) {
  visualizer.add_network_info(client.network_info());
  if let Ok(ctx) = egui_contexts.ctx_mut() {
    visualizer.show_window(ctx);
  } else {
    warn!("Failed to get Egui context for Renet client visualiser");
  }
}

/// System that checks whether the client completed the handshake before the deadline.
/// If the handshake did not complete in time, it cleans up the client transport and
/// emits a UI error message.
pub fn client_handshake_system(
  mut commands: Commands,
  client: Res<RenetClient>,
  handshake: Option<Res<PendingClientHandshake>>,
  mut ui_message_writer: MessageWriter<UiNotification>,
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
    ui_message_writer.write(UiNotification::error(message));
    commands.remove_resource::<RenetClient>();
    commands.remove_resource::<NetcodeClientTransport>();
    commands.remove_resource::<PendingClientHandshake>();
    commands.remove_resource::<RenetClientVisualizer<VISUALISER_DISPLAY_VALUES>>()
  }
}

/// Processes any incoming [`DefaultChannel::ReliableOrdered`] server messages and acts on them, if required.
fn receive_reliable_server_messages_system(
  mut client: ResMut<RenetClient>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut registration_message: MessageWriter<PlayerRegistrationMessage>,
  mut exit_lobby_message: MessageWriter<ExitLobbyMessage>,
  current_state: ResMut<State<AppState>>,
  mut next_state: ResMut<NextState<AppState>>,
  mut winner: ResMut<WinnerInfo>,
  mut seed: ResMut<Seed>,
) {
  while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
    let server_message = decode_from_bytes(&message).expect("Failed to deserialise server message");
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
        utils::register_player_locally(
          &mut registered_players,
          &available_configs,
          &mut registration_message,
          player_id,
        );
      }
      ServerMessage::PlayerUnregistered { player_id, .. } => {
        let player_id = PlayerId(player_id);
        utils::unregister_player_locally(&mut registered_players, &mut registration_message, player_id);
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
      ServerMessage::ShutdownServer => {
        exit_lobby_message.write(ExitLobbyMessage::forced_by_server());
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
    let message = encode_to_bytes(&client_message).expect("Failed to serialise player registration message");
    client.send_message(DefaultChannel::ReliableOrdered, message);
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
  mut messages: MessageReader<SerialisableInputActionMessage>,
  registered_players: Res<RegisteredPlayers>,
  mut client: ResMut<RenetClient>,
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
      if let Ok(input_message) = encode_to_bytes(&ClientMessage::Input(*message)) {
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
    if let Ok(server_message) = decode_from_bytes(&message) {
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
  let domain_width = RESOLUTION_WIDTH as f32 + 2. * WRAPAROUND_MARGIN;
  let domain_height = RESOLUTION_HEIGHT as f32 + 2. * WRAPAROUND_MARGIN;

  for (mut transform, interpolation, _) in snake_head_query.iter_mut() {
    let current_position = transform.translation.truncate();
    let mut target_position = interpolation.target_position;

    // If the difference between current and target X position is big enough to indicate wraparound,
    // adjust the target position to keep going and let the wraparound happen in the wraparound system
    let dx = target_position.x - current_position.x;
    if dx.abs() > domain_width / 2. {
      if dx > 0. {
        target_position.x -= domain_width;
      } else {
        target_position.x += domain_width;
      }
    }

    // If the difference between current and target Y position is big enough to indicate wraparound,
    // adjust the target position to keep going and let the wraparound happen in the wraparound system
    let dy = target_position.y - current_position.y;
    if dy.abs() > domain_height / 2. {
      if dy > 0. {
        target_position.y -= domain_height;
      } else {
        target_position.y += domain_height;
      }
    }

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
  use crate::online::lib::NetworkingMessagesPlugin;
  use crate::prelude::{SharedMessagesPlugin, SharedResourcesPlugin};
  use bevy::math::Vec3;
  use bevy::prelude::*;
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
  fn apply_state_interpolation_system_ignores_post_wraparound_position() {
    let mut app = setup();

    // Spawn an entity that the system will operate on
    let entity = app
      .world_mut()
      .spawn((
        Transform::from_translation(Vec3::new(RESOLUTION_WIDTH as f32, 100., 0.)),
        NetworkTransformInterpolation::new(2.),
        PlayerId(1),
        SnakeHead,
      ))
      .id();

    // Add the system to be tested and the message to be processed inside the system
    app.add_systems(Update, apply_state_interpolation_system);
    app
      .world_mut()
      .write_message(PlayerStateUpdateMessage::new(1, (0., 100.), 0.))
      .expect("Failed to write PlayerStateUpdateMessage message");
    app.update();

    // Advance the time a little
    advance_time_by(&mut app, Duration::from_millis(100));

    let translation = app.world_mut().get::<Transform>(entity).unwrap().translation;
    let epsilon = 1e-4_f32; // Tolerance for floating point rounding
    assert!(
      (translation.y - 100.0).abs() <= epsilon,
      "Y position differs by more than {}: {}",
      epsilon,
      translation.y
    );
    assert_ne!(translation.x, 0.);
    assert!(
      translation.x > RESOLUTION_WIDTH as f32,
      "X position did not advance beyond during interpolation"
    );
  }
}
