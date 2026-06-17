use crate::app_state::AppState;
use crate::online::structs::{LocalInputMapping, NetworkTransformInterpolation};
use crate::online::utils;
use crate::prelude::{
  AvailableControlSchemes, ControlSchemeId, ExitLobbyMessage, InputMessage, LocalPlayerRegistrationRequestMessage,
  MenuName, PlayerId, PlayerName, PlayerRegistrationMessage, RegisteredPlayers, Seed, SnakeHead, ToggleMenuMessage,
  WinnerInfo,
};
use bevy::app::Update;
use bevy::log::{debug, error_once, info, warn};
use bevy::math::Quat;
use bevy::prelude::{
  App, Commands, Entity, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, OnExit, Plugin, Query, Res,
  ResMut, Resource, State, Time, Transform, With, Without, in_state, resource_exists,
};
use mooplas_networking::prelude::{
  ChannelType, ClientId, ClientMessage, ClientNetworkingActive, InboundServerMessage, OutboundClientMessage,
  PlayerStateUpdateMessage, SerialisableRegisteredPlayer, SerialisableRegistrationRequest,
  SerialisableUnregistrationRequest, encode_to_bytes,
};

/// A plugin that adds shared client-side online multiplayer capabilities to the game. Contains systems that are shared
/// between different client implementations.
pub struct ClientPlugin;

#[derive(Resource, Default)]
struct CurrentClientId(Option<ClientId>);

#[derive(Resource)]
struct PendingClientBootstrap {
  target_state: AppState,
  registered_players: Vec<SerialisableRegisteredPlayer>,
  winner_info: Option<u8>,
}

impl Plugin for ClientPlugin {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<CurrentClientId>()
      .add_systems(
        Update,
        handle_inbound_server_message.run_if(resource_exists::<ClientNetworkingActive>),
      )
      .add_systems(OnExit(AppState::Initialising), apply_pending_client_bootstrap_system)
      .add_systems(
        Update,
        (
          handle_local_player_registration_request_message,
          handle_local_exit_lobby_message,
        )
          .run_if(in_state(AppState::Registering))
          .run_if(resource_exists::<ClientNetworkingActive>),
      )
      .add_systems(
        Update,
        (
          send_local_input_messages,
          add_interpolation_component_system,
          apply_state_interpolation_system,
        )
          .run_if(in_state(AppState::Playing))
          .run_if(resource_exists::<ClientNetworkingActive>),
      );
  }
}

fn register_player_locally(
  mut registered_players: &mut ResMut<RegisteredPlayers>,
  available_control_schemes: &Res<AvailableControlSchemes>,
  mut local_input_mapping: &mut ResMut<LocalInputMapping>,
  current_client_id: &CurrentClientId,
  mut registration_message: &mut MessageWriter<PlayerRegistrationMessage>,
  client_id: &ClientId,
  player_id: &u8,
  control_scheme_id: &u8,
  name: &String,
) {
  let player_id = PlayerId(*player_id);
  let control_scheme_id = ControlSchemeId(*control_scheme_id);
  if current_client_id.0 == Some(*client_id) {
    utils::register_local_player_locally(
      &mut registered_players,
      &available_control_schemes,
      &mut registration_message,
      Some(&mut local_input_mapping),
      player_id,
      control_scheme_id,
      name.clone(),
    );
  } else {
    utils::register_remote_player_locally(
      &mut registered_players,
      &available_control_schemes,
      &mut registration_message,
      player_id,
      control_scheme_id,
      name.clone(),
    );
  }
}

fn set_state_safely(current_state: AppState, next_state: &mut ResMut<NextState<AppState>>, target_state: AppState) {
  if is_state_transition_permitted(&current_state, &target_state) {
    next_state.set(target_state);
  } else {
    debug!(
      "Ignoring state change to [{}] because [{}] is restricted or unchanged...",
      target_state, current_state
    );
  }
}

/// Returns `true` if the current state is considered to be restricted. This includes states that the application
/// automatically transitions to. Used to stop the server in a multiplayer context from causing an inconsistent state.
/// - `Initialising`: It is not allowed to transition *out of* this state "manually" because this happens
///   automatically once initialisation is complete.
/// - `Registering`: It is not allowed to transition *into* this state "manually" because this state is automatically
///   transitioned into from `Initialising`. You must transition to `Initialising` instead.
/// - Transitions from one state to itself are also restricted.
fn is_state_transition_permitted(from_state: &AppState, to_state: &AppState) -> bool {
  !matches!(from_state, AppState::Initialising) && !matches!(to_state, AppState::Registering) && from_state != to_state
}

/// Processes any incoming server messages and acts on them, if required.
fn handle_inbound_server_message(
  mut commands: Commands,
  mut messages: MessageReader<InboundServerMessage>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_control_schemes: Res<AvailableControlSchemes>,
  mut local_input_mapping: ResMut<LocalInputMapping>,
  current_state: Res<State<AppState>>,
  mut next_state: ResMut<NextState<AppState>>,
  mut current_client_id: ResMut<CurrentClientId>,
  mut winner: ResMut<WinnerInfo>,
  mut seed: ResMut<Seed>,
  mut registration_message: MessageWriter<PlayerRegistrationMessage>,
  mut player_state_update_message: MessageWriter<PlayerStateUpdateMessage>,
  mut exit_lobby_message: MessageWriter<ExitLobbyMessage>,
) {
  for message in messages.read() {
    match message {
      InboundServerMessage::ClientConnected { client_id } => info!("[{:?}] connected", client_id),
      InboundServerMessage::ClientDisconnected { client_id } => info!("[{:?}] disconnected", client_id),
      InboundServerMessage::ClientInitialised {
        seed: server_seed,
        client_id,
        current_state: server_state,
        registered_players: server_registered_players,
        winner_info,
      } => {
        seed.set(*server_seed);
        current_client_id.0 = Some(*client_id);
        registered_players.clear();
        local_input_mapping.clear();
        commands.insert_resource(PendingClientBootstrap {
          target_state: AppState::from(server_state),
          registered_players: server_registered_players.clone(),
          winner_info: *winner_info,
        });
        if *current_state != AppState::Initialising {
          next_state.set(AppState::Initialising);
        }
      }
      InboundServerMessage::PlayerRegistered {
        client_id,
        player_id,
        control_scheme_id,
        name,
      } => {
        register_player_locally(
          &mut registered_players,
          &available_control_schemes,
          &mut local_input_mapping,
          &mut current_client_id,
          &mut registration_message,
          client_id,
          player_id,
          control_scheme_id,
          name,
        );
      }
      InboundServerMessage::PlayerUnregistered { client_id, player_id } => {
        let player_id = PlayerId(*player_id);
        if current_client_id.0 == Some(*client_id) {
          utils::unregister_local_player_locally(
            &mut registered_players,
            &mut registration_message,
            Some(&mut local_input_mapping),
            player_id,
          );
        } else {
          utils::unregister_remote_player_locally(&mut registered_players, &mut registration_message, player_id);
        }
      }
      InboundServerMessage::StateChanged { new_state, winner_info } => {
        let new_state = AppState::from(new_state);
        set_state_safely(*current_state.get(), &mut next_state, new_state);
        if let Some(player_id) = winner_info {
          winner.set((*player_id).into());
        }
      }
      InboundServerMessage::ShutdownServer => {
        exit_lobby_message.write(ExitLobbyMessage::forced_by_server());
      }
      InboundServerMessage::UpdatePlayerStates { states } => {
        for (player_id, x, y, rotation_z) in states {
          player_state_update_message.write(PlayerStateUpdateMessage::new(*player_id, (*x, *y), *rotation_z));
        }
      }
    }
  }
}

// TODO: Check if there are better ways to run this system conditionally
fn apply_pending_client_bootstrap_system(
  mut commands: Commands,
  pending_bootstrap: Option<Res<PendingClientBootstrap>>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_control_schemes: Res<AvailableControlSchemes>,
  mut local_input_mapping: ResMut<LocalInputMapping>,
  current_client_id: Res<CurrentClientId>,
  mut winner: ResMut<WinnerInfo>,
  mut registration_message: MessageWriter<PlayerRegistrationMessage>,
  current_state: Res<State<AppState>>,
  mut next_state: ResMut<NextState<AppState>>,
) {
  let Some(pending_bootstrap) = pending_bootstrap else {
    return;
  };

  for player in &pending_bootstrap.registered_players {
    register_player_locally(
      &mut registered_players,
      &available_control_schemes,
      &mut local_input_mapping,
      &*current_client_id,
      &mut registration_message,
      &player.client_id,
      &player.player_id,
      &player.control_scheme_id,
      &player.name,
    );
  }
  if let Some(player_id) = pending_bootstrap.winner_info {
    winner.set(player_id.into());
  }
  if pending_bootstrap.target_state != AppState::Registering {
    next_state.set(pending_bootstrap.target_state);
  }
  set_state_safely(**current_state, &mut next_state, pending_bootstrap.target_state);
  commands.remove_resource::<PendingClientBootstrap>();
}

/// A system that handles local player registration requests by sending them to the server.
fn handle_local_player_registration_request_message(
  mut messages: MessageReader<LocalPlayerRegistrationRequestMessage>,
  local_input_mapping: Res<LocalInputMapping>,
  mut outbound_client_message: MessageWriter<OutboundClientMessage>,
  player_name: Res<PlayerName>,
) {
  for request in messages.read() {
    let client_message = if request.has_registered {
      ClientMessage::RegistrationRequest(SerialisableRegistrationRequest {
        control_scheme_id: request.control_scheme_id.0,
        name: player_name.get().to_string(),
      })
    } else {
      let Some(player_id) = local_input_mapping.get_player_id(&request.control_scheme_id) else {
        warn!(
          "Skipping unregistration request for unknown local control scheme [{:?}]",
          request.control_scheme_id
        );
        continue;
      };
      ClientMessage::UnregistrationRequest(SerialisableUnregistrationRequest {
        player_id: player_id.into(),
      })
    };
    debug!("Sending: [{:?}]", client_message);
    let payload = encode_to_bytes(&client_message).expect("Failed to serialise player registration message");
    outbound_client_message.write(OutboundClientMessage::Send {
      channel: ChannelType::ReliableOrdered,
      payload,
    });
  }
}

/// A system that handles local exit lobby messages by disconnecting from the server and returning to the main menu.
fn handle_local_exit_lobby_message(
  mut messages: MessageReader<ExitLobbyMessage>,
  mut outbound_client_message: MessageWriter<OutboundClientMessage>,
  mut toggle_menu_message: MessageWriter<ToggleMenuMessage>,
  mut registered_players: ResMut<RegisteredPlayers>,
  mut local_input_mapping: ResMut<LocalInputMapping>,
  mut current_client_id: ResMut<CurrentClientId>,
) {
  for message in messages.read() {
    debug!("Disconnecting from server (by force={})...", message.by_force);
    if !message.by_force {
      outbound_client_message.write(OutboundClientMessage::Disconnect);
    }
    toggle_menu_message.write(ToggleMenuMessage::set(MenuName::MainMenu));
    registered_players.clear();
    local_input_mapping.clear();
    current_client_id.0 = None;
  }
}

/// A system that handles local input action messages for mutable players by sending them to the server in order to sync
/// the movements of the local player(s) with the server.
fn send_local_input_messages(
  mut messages: MessageReader<InputMessage>,
  registered_players: Res<RegisteredPlayers>,
  mut outbound_client_message: MessageWriter<OutboundClientMessage>,
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
      if let Ok(payload) = encode_to_bytes(&ClientMessage::Input(message.into())) {
        outbound_client_message.write(OutboundClientMessage::Send {
          channel: ChannelType::Unreliable,
          payload,
        });
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
  use crate::app_state::AppStatePlugin;
  use crate::initialisation::InitialisationPlugin;
  use crate::prelude::constants::RESOLUTION_WIDTH;
  use crate::prelude::{SharedMessagesPlugin, SharedResourcesPlugin};
  use bevy::math::Vec3;
  use bevy::prelude::*;
  use bevy::state::app::StatesPlugin;
  use mooplas_networking::prelude::NetworkingMessagesPlugin;
  use std::time::Duration;

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Add shared messages and resources as they are required by the game loop systems
    app.add_plugins((
      StatesPlugin,
      AppStatePlugin,
      SharedMessagesPlugin,
      SharedResourcesPlugin,
      NetworkingMessagesPlugin,
    ));
    app
      .init_resource::<CurrentClientId>()
      .init_resource::<LocalInputMapping>();
    app
  }

  fn add_control_schemes(app: &mut App, count: u8) {
    let mut available_control_schemes = app
      .world_mut()
      .get_resource_mut::<AvailableControlSchemes>()
      .expect("AvailableControlSchemes resource missing");
    for id in 0..count {
      available_control_schemes
        .schemes
        .push(crate::prelude::ControlScheme::test(id));
    }
  }

  fn set_app_state(app: &mut App, state: AppState) {
    app.world_mut().resource_mut::<NextState<AppState>>().set(state);
    app.update();
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

  #[test]
  fn handle_inbound_server_message_does_not_allow_late_joiner_to_enter_registering_directly() {
    let mut app = setup();
    set_app_state(&mut app, AppState::Preparing);
    app.add_systems(Update, handle_inbound_server_message);

    app
      .world_mut()
      .write_message(InboundServerMessage::StateChanged {
        new_state: "Registering".to_string(),
        winner_info: None,
      })
      .expect("Failed to write StateChanged message");
    app.update();
    app.update();

    let state = app.world().resource::<State<AppState>>();
    assert_eq!(state.get(), &AppState::Preparing);
  }

  #[test]
  fn handle_inbound_server_message_does_not_advance_seed_before_server_driven_initialising() {
    let mut app = setup();
    set_app_state(&mut app, AppState::Registering);
    app.world_mut().resource_mut::<Seed>().set(41);
    app.add_systems(Update, handle_inbound_server_message);

    app
      .world_mut()
      .write_message(InboundServerMessage::StateChanged {
        new_state: "Initialising".to_string(),
        winner_info: None,
      })
      .expect("Failed to write StateChanged message");
    app.update();

    assert_eq!(app.world().resource::<Seed>().get(), 41);
  }

  #[test]
  fn handle_inbound_server_message_sets_seed_when_client_initialised_received() {
    let mut app = setup();
    app.world_mut().resource_mut::<Seed>().set(41);
    app.add_systems(Update, handle_inbound_server_message);
    set_app_state(&mut app, AppState::Preparing);

    app
      .world_mut()
      .write_message(InboundServerMessage::ClientInitialised {
        seed: 123,
        client_id: ClientId::from_renet_u64(7),
        current_state: "Registering".to_string(),
        registered_players: Vec::new(),
        winner_info: None,
      })
      .expect("Failed to write ClientInitialised message");
    app
      .world_mut()
      .write_message(InboundServerMessage::StateChanged {
        new_state: "Initialising".to_string(),
        winner_info: None,
      })
      .expect("Failed to write StateChanged message");

    app.update();

    assert_eq!(app.world().resource::<Seed>().get(), 123);
  }

  #[test]
  fn client_initialised_bootstrap_runs_initialising_before_registering() {
    let mut app = setup();
    app.add_plugins((InitialisationPlugin, ClientPlugin));
    app.insert_resource(ClientNetworkingActive);
    set_app_state(&mut app, AppState::Preparing);

    let client_id = ClientId::from_renet_u64(7);
    let host_client_id = ClientId::nil();
    app
      .world_mut()
      .write_message(InboundServerMessage::ClientInitialised {
        seed: 123,
        client_id,
        current_state: "Registering".to_string(),
        registered_players: vec![SerialisableRegisteredPlayer {
          client_id: host_client_id,
          player_id: 0,
          control_scheme_id: 0,
          name: "Host".to_string(),
        }],
        winner_info: None,
      })
      .expect("Failed to write ClientInitialised message");

    for _ in 0..4 {
      app.update();
    }

    let state = app.world().resource::<State<AppState>>();
    assert_eq!(state.get(), &AppState::Registering);
    let available_control_schemes = app.world().resource::<AvailableControlSchemes>();
    assert!(!available_control_schemes.schemes.is_empty());
    let actual_spawn_points = app.world().resource::<crate::prelude::SpawnPoints>();
    assert_eq!(actual_spawn_points.data.len(), 10);
    assert_eq!(app.world().resource::<Seed>().get(), 123);
    let registered_players = app.world().resource::<RegisteredPlayers>();
    let player = registered_players
      .players
      .iter()
      .find(|player| player.id == PlayerId(0))
      .expect("Expected bootstrapped host player");
    assert!(player.is_remote());
    assert_eq!(player.name, "Host");
  }

  #[test]
  fn handle_inbound_server_message_registers_own_player_locally_from_assigned_player_id() {
    let mut app = setup();
    add_control_schemes(&mut app, 1);
    app.add_systems(Update, handle_inbound_server_message);

    let client_id = ClientId::from_renet_u64(7);
    app
      .world_mut()
      .write_message(InboundServerMessage::ClientInitialised {
        seed: 123,
        client_id,
        current_state: "Registering".to_string(),
        registered_players: Vec::new(),
        winner_info: None,
      })
      .expect("Failed to write ClientInitialised message");
    app.update();

    app
      .world_mut()
      .write_message(InboundServerMessage::PlayerRegistered {
        client_id,
        player_id: 4,
        control_scheme_id: 0,
        name: "Test".to_string(),
      })
      .expect("Failed to write PlayerRegistered message");
    app.update();

    let registered_players = app.world().resource::<RegisteredPlayers>();
    let player = registered_players
      .players
      .iter()
      .find(|player| player.id == PlayerId(4))
      .expect("Expected own player to be registered locally");
    assert!(player.is_local());
    assert_eq!(player.input.id, ControlSchemeId(0));

    let local_input_mapping = app.world().resource::<LocalInputMapping>();
    assert_eq!(
      local_input_mapping.get_player_id(&ControlSchemeId(0)),
      Some(PlayerId(4))
    );
  }
}
