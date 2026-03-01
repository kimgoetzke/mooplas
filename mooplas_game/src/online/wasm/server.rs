use crate::app_state::AppState;
use crate::online::utils;
use crate::prelude::{
  AvailablePlayerConfigs, ExitLobbyMessage, InputMessage, MenuName, PlayerId, PlayerRegistrationMessage,
  RegisteredPlayers, Seed, SnakeHead, ToggleMenuMessage, WinnerInfo,
};
use bevy::app::{App, Plugin, Update};
use bevy::log::{debug, info, warn};
use bevy::prelude::{
  Commands, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, On, Query, Res, ResMut, Resource,
  StateTransitionEvent, Time, Timer, TimerMode, Transform, With, in_state, resource_exists,
};
use mooplas_networking::prelude::{
  ChannelType, ClientEvent, ClientId, Lobby, NetworkRole, OutgoingServerMessage, ServerEvent, ServerNetworkingActive,
  encode_to_bytes,
};
use mooplas_networking_matchbox::prelude::ServerMatchboxPlugin;
use std::time::Duration;

/// A plugin that adds server-side online multiplayer capabilities to the game. Only active when the game is running in
/// server mode. Mutually exclusive with the [`crate::online::ClientPlugin`].
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(ServerMatchboxPlugin)
      .add_observer(receive_server_events)
      .add_observer(receive_client_events)
      .add_systems(
        Update,
        broadcast_local_app_state_system.run_if(resource_exists::<ServerNetworkingActive>),
      )
      .add_systems(
        Update,
        (
          broadcast_local_player_registration_system,
          process_and_broadcast_local_exit_lobby_message,
        )
          .run_if(in_state(AppState::Registering))
          .run_if(resource_exists::<ServerNetworkingActive>),
      )
      .add_systems(
        Update,
        broadcast_player_states_system
          .run_if(in_state(AppState::Playing))
          .run_if(resource_exists::<ServerNetworkingActive>),
      )
      .add_systems(
        Update,
        disconnect_all_clients_system
          .run_if(resource_exists::<ShutdownCountdown>)
          .run_if(resource_exists::<ServerNetworkingActive>),
      );
  }
}

// A resource to schedule the actual disconnect after broadcasting the shutdown message.
#[derive(Resource)]
struct ShutdownCountdown(Timer);

const CLIENT_MESSAGE_SERIALISATION: &'static str = "Failed to serialise client message";

/// The main observer system for server events.
fn receive_server_events(
  server_event: On<ServerEvent>,
  mut outgoing_messages: MessageWriter<OutgoingServerMessage>,
  mut lobby: ResMut<Lobby>,
  mut next_state: ResMut<NextState<AppState>>,
  seed: Res<Seed>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut player_registration_message: MessageWriter<PlayerRegistrationMessage>,
) {
  match server_event.event() {
    ServerEvent::ClientConnected { client_id } => {
      info!("Client with ID [{}] connected", client_id);

      // TODO: Communicate current state of the lobby (registered players, etc.) to the newly connected client
      // Send the current seed to the newly connected client
      let seed_message = encode_to_bytes(&ServerEvent::ClientInitialised {
        seed: seed.get(),
        client_id: *client_id,
      })
      .expect("Failed to serialise seed message");
      outgoing_messages.write(OutgoingServerMessage::Send {
        client_id: *client_id,
        channel: ChannelType::ReliableOrdered,
        payload: seed_message,
      });
    }
    ServerEvent::ClientDisconnected { client_id } => {
      info!("Client with ID [{}] disconnected", client_id);

      // Unregister any players associated with this client and notify other clients about it
      for player_id in lobby.get_registered_players_cloned(&client_id).iter() {
        handle_player_registration_message_from_client(
          &mut outgoing_messages,
          &mut registered_players,
          &available_configs,
          &mut player_registration_message,
          &client_id,
          *player_id,
          false,
          &mut lobby,
        );
      }
    }
    _ => { /* Ignored */ }
  }

  // TODO: Improve state transition logic
  if lobby.connected.len() > 0 {
    next_state.set(AppState::Initialising);
  }
}

/// Processes any incoming messages from clients by applying them locally and broadcasting them to all other clients,
/// if necessary.
fn receive_client_events(
  client_event: On<ClientEvent>,
  mut lobby: ResMut<Lobby>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut player_registration_message: MessageWriter<PlayerRegistrationMessage>,
  mut input_message: MessageWriter<InputMessage>,
  mut outgoing_messages: MessageWriter<OutgoingServerMessage>,
) {
  match client_event.event() {
    ClientEvent::PlayerRegistration(message, client_id) => {
      handle_player_registration_message_from_client(
        &mut outgoing_messages,
        &mut registered_players,
        &available_configs,
        &mut player_registration_message,
        &client_id,
        message.player_id,
        message.has_registered,
        &mut lobby,
      );
    }
    ClientEvent::Input(message, client_id) => {
      let message: InputMessage = message.into();
      let player_id = match message {
        InputMessage::Action(player_id) => player_id,
        InputMessage::Move(player_id, _) => player_id,
      };
      if lobby.validate_registration(&client_id, &player_id.into()) {
        input_message.write(message);
        return;
      }
      warn!("Received invalid input action on [Unreliable] channel: {:?}", message);
    }
  }
}

/// Broadcasts the authoritative state (position and rotation) of all snake heads to all clients.
/// This runs every frame to ensure clients have up-to-date positions for interpolation.
fn broadcast_player_states_system(
  mut outgoing_messages: MessageWriter<OutgoingServerMessage>,
  snake_heads: Query<(&Transform, &PlayerId), With<SnakeHead>>,
) {
  let mut states = Vec::new();
  for (transform, player_id) in snake_heads.iter() {
    let position = transform.translation;
    let (_, _, rotation_z) = transform.rotation.to_euler(bevy::math::EulerRot::XYZ);
    states.push((player_id.0, position.x, position.y, rotation_z));
  }

  if states.is_empty() {
    return;
  }

  if let Ok(payload) = encode_to_bytes(&ServerEvent::UpdatePlayerStates { states }) {
    outgoing_messages.write(OutgoingServerMessage::Broadcast {
      channel: ChannelType::Unreliable,
      payload,
    });
  } else {
    warn!("Failed to serialise player states message");
  }
}

/// Processes an individual player registration message from [`ClientId`].
fn handle_player_registration_message_from_client(
  outgoing_messages: &mut MessageWriter<OutgoingServerMessage>,
  mut registered_players: &mut ResMut<RegisteredPlayers>,
  available_configs: &Res<AvailablePlayerConfigs>,
  mut player_registration_message: &mut MessageWriter<PlayerRegistrationMessage>,
  client_id: &ClientId,
  player_id: mooplas_networking::prelude::PlayerId,
  has_registered: bool,
  lobby: &mut ResMut<Lobby>,
) {
  if has_registered {
    info!("[{}] with client ID [{}] registered", player_id, client_id);
    let payload = encode_to_bytes(&ServerEvent::PlayerRegistered {
      client_id: (*client_id).into(),
      player_id: player_id.0,
    })
    .expect(CLIENT_MESSAGE_SERIALISATION);
    outgoing_messages.write(OutgoingServerMessage::BroadcastExcept {
      except_client_id: *client_id,
      channel: ChannelType::ReliableOrdered,
      payload,
    });
    utils::register_player_locally(
      &mut registered_players,
      &available_configs,
      &mut player_registration_message,
      player_id.into(),
    );
    lobby.register_player(*client_id, player_id.into());
  } else {
    info!("[{}] with client ID [{}] unregistered", player_id, client_id);
    let payload = encode_to_bytes(&ServerEvent::PlayerUnregistered {
      client_id: (*client_id).into(),
      player_id: player_id.0,
    })
    .expect(CLIENT_MESSAGE_SERIALISATION);
    outgoing_messages.write(OutgoingServerMessage::BroadcastExcept {
      except_client_id: *client_id,
      channel: ChannelType::ReliableOrdered,
      payload,
    });
    utils::unregister_player_locally(
      &mut registered_players,
      &mut player_registration_message,
      player_id.into(),
    );
    lobby.unregister_player(*client_id, player_id);
  }
}

/// A system that handles local state change events and broadcasts them to all connected clients.
fn broadcast_local_app_state_system(
  mut outgoing_messages: MessageWriter<OutgoingServerMessage>,
  mut app_state_messages: MessageReader<StateTransitionEvent<AppState>>,
  winner: Res<WinnerInfo>,
) {
  for message in app_state_messages.read() {
    if let Some(state_name) = message.entered {
      let server_event = ServerEvent::StateChanged {
        new_state: state_name.to_string(),
        winner_info: winner.get_as_u8(),
      };
      debug!("Broadcasting: {:?}", server_event);
      if let Ok(payload) = encode_to_bytes(&server_event) {
        outgoing_messages.write(OutgoingServerMessage::Broadcast {
          channel: ChannelType::ReliableOrdered,
          payload,
        });
      } else {
        warn!("{}: {:?}", CLIENT_MESSAGE_SERIALISATION, server_event);
        return;
      }
    }
  }
}

/// A system that handles local messages (such as player registration messages) and broadcasts them to all connected
/// clients.
fn broadcast_local_player_registration_system(
  mut messages: MessageReader<PlayerRegistrationMessage>,
  mut outgoing_messages: MessageWriter<OutgoingServerMessage>,
) {
  for message in messages.read() {
    if utils::should_message_be_skipped(&message, NetworkRole::Client) {
      continue;
    }
    if message.has_registered {
      debug!("Broadcasting: [{}] registered locally...", message.player_id);
      let payload = encode_to_bytes(&ServerEvent::PlayerRegistered {
        client_id: ClientId::from_renet_u64(0),
        player_id: message.player_id.0,
      })
      .expect(CLIENT_MESSAGE_SERIALISATION);
      outgoing_messages.write(OutgoingServerMessage::Broadcast {
        channel: ChannelType::ReliableOrdered,
        payload,
      });
    } else {
      debug!("Broadcasting: [{}] unregistered locally...", message.player_id);
      let payload = encode_to_bytes(&ServerEvent::PlayerUnregistered {
        client_id: ClientId::from_renet_u64(0),
        player_id: message.player_id.0,
      })
      .expect(CLIENT_MESSAGE_SERIALISATION);
      outgoing_messages.write(OutgoingServerMessage::Broadcast {
        channel: ChannelType::ReliableOrdered,
        payload,
      });
    }
  }
}

/// A system that processes local exit lobby messages and broadcasts the servers intention to shut down to all connected
/// clients. This will then schedule the actual disconnect after a short delay.
fn process_and_broadcast_local_exit_lobby_message(
  mut messages: MessageReader<ExitLobbyMessage>,
  mut commands: Commands,
  mut outgoing_messages: MessageWriter<OutgoingServerMessage>,
) {
  for _ in messages.read() {
    info!("Informing all clients about intention to shut down server and scheduling shutdown...");
    let payload = encode_to_bytes(&ServerEvent::ShutdownServer).expect(CLIENT_MESSAGE_SERIALISATION);
    outgoing_messages.write(OutgoingServerMessage::Broadcast {
      channel: ChannelType::ReliableOrdered,
      payload,
    });
    commands.insert_resource(ShutdownCountdown(Timer::new(
      Duration::from_millis(500),
      TimerMode::Once,
    )));
  }
}

/// Runs while [`ShutdownCountdown`] exists. When the timer finishes, all clients are disconnected, all networking
/// related resources are cleared, and the app state is set to [`AppState::Preparing`].
fn disconnect_all_clients_system(
  mut commands: Commands,
  mut countdown: ResMut<ShutdownCountdown>,
  time: Res<Time>,
  mut lobby: ResMut<Lobby>,
  mut registered_players: ResMut<RegisteredPlayers>,
  mut toggle_menu_message: MessageWriter<ToggleMenuMessage>,
  mut next_app_state: ResMut<NextState<AppState>>,
  mut outgoing_messages: MessageWriter<OutgoingServerMessage>,
) {
  countdown.0.tick(time.delta());
  if countdown.0.just_finished() {
    info!("Disconnecting all clients now...");
    lobby.clear();
    registered_players.clear();
    outgoing_messages.write(OutgoingServerMessage::DisconnectAll);
    commands.remove_resource::<ShutdownCountdown>();
    toggle_menu_message.write(ToggleMenuMessage::set(MenuName::MainMenu));
    next_app_state.set(AppState::Preparing);
  }
}
