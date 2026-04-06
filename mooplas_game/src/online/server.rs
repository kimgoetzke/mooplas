use crate::app_state::AppState;
use crate::online::utils;
use crate::prelude::{
  AvailableControlSchemes, ControlSchemeId, ExitLobbyMessage, InputMessage, LocalPlayerRegistrationRequestMessage,
  MAX_PLAYERS, MenuName, PlayerId, PlayerRegistrationMessage, RegisteredPlayers, Seed, SnakeHead, ToggleMenuMessage,
  WinnerInfo,
};
use bevy::log::{debug, info, warn};
use bevy::prelude::{
  App, Commands, IntoScheduleConfigs, MessageReader, MessageWriter, NextState, Plugin, Query, Res, ResMut, Resource,
  State, StateTransitionEvent, Time, Timer, TimerMode, Transform, Update, With, in_state, resource_exists,
};
use mooplas_networking::prelude::{
  ChannelType, ClientId, InboundClientMessage, InboundServerMessage, Lobby, OutboundServerMessage,
  SerialisableUnregistrationRequest, ServerNetworkingActive, encode_to_bytes,
};
use std::time::Duration;

/// A plugin that contains systems related to processing and broadcasting messages on the server, which are shared
/// between different server implementations.
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Update,
        (
          handle_inbound_client_message,
          handle_inbound_server_message,
          handle_local_state_transition_event,
        )
          .run_if(resource_exists::<ServerNetworkingActive>),
      )
      .add_systems(
        Update,
        (
          handle_local_player_registration_request_message,
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

const CLIENT_MESSAGE_SERIALISATION: &str = "Failed to serialise client message";

// A resource to schedule the actual disconnect after broadcasting the shutdown message.
#[derive(Resource)]
struct ShutdownCountdown(Timer);

fn host_client_id() -> ClientId {
  ClientId::nil()
}

fn broadcast_player_registered(
  outbound_server_message: &mut MessageWriter<OutboundServerMessage>,
  client_id: ClientId,
  player_id: PlayerId,
  control_scheme_id: ControlSchemeId,
) {
  let payload = encode_to_bytes(&InboundServerMessage::PlayerRegistered {
    client_id,
    player_id: player_id.0,
    control_scheme_id: control_scheme_id.0,
  })
  .expect(CLIENT_MESSAGE_SERIALISATION);
  outbound_server_message.write(OutboundServerMessage::Broadcast {
    channel: ChannelType::ReliableOrdered,
    payload,
  });
}

fn broadcast_player_unregistered(
  outbound_server_message: &mut MessageWriter<OutboundServerMessage>,
  client_id: ClientId,
  player_id: PlayerId,
) {
  let payload = encode_to_bytes(&InboundServerMessage::PlayerUnregistered {
    client_id,
    player_id: player_id.0,
  })
  .expect(CLIENT_MESSAGE_SERIALISATION);
  outbound_server_message.write(OutboundServerMessage::Broadcast {
    channel: ChannelType::ReliableOrdered,
    payload,
  });
}

fn handle_registration_request(
  outbound_server_message: &mut MessageWriter<OutboundServerMessage>,
  registered_players: &mut ResMut<RegisteredPlayers>,
  available_control_schemes: &Res<AvailableControlSchemes>,
  player_registration_message: &mut MessageWriter<PlayerRegistrationMessage>,
  client_id: ClientId,
  control_scheme_id: ControlSchemeId,
  lobby: &mut ResMut<Lobby>,
  registers_local_player: bool,
) {
  if lobby.is_control_scheme_registered(&client_id, control_scheme_id.0) {
    warn!(
      "Ignoring duplicate registration for client [{}] and control scheme [{:?}]",
      client_id, control_scheme_id
    );
    return;
  }
  let Some(player_id) = next_available_player_id(registered_players) else {
    warn!("No player IDs are available for client [{}]", client_id);
    return;
  };

  info!(
    "[{}] with client ID [{}] registered using control scheme [{:?}]",
    player_id, client_id, control_scheme_id
  );

  if registers_local_player {
    utils::register_local_player_locally(
      registered_players,
      available_control_schemes,
      player_registration_message,
      None,
      player_id,
      control_scheme_id,
    );
  } else {
    utils::register_remote_player_locally(
      registered_players,
      available_control_schemes,
      player_registration_message,
      player_id,
      control_scheme_id,
    );
  }

  lobby.register_player(client_id, player_id.into(), control_scheme_id.0);
  broadcast_player_registered(outbound_server_message, client_id, player_id, control_scheme_id);
}

/// Returns the first [`PlayerId`] that is not registered, or `None` if all possible player IDs are taken.
fn next_available_player_id(registered_players: &RegisteredPlayers) -> Option<PlayerId> {
  (0..MAX_PLAYERS as usize)
    .map(|index| PlayerId(index as u8))
    .find(|candidate| !registered_players.players.iter().any(|player| player.id == *candidate))
}

fn handle_unregistration_request(
  outbound_server_message: &mut MessageWriter<OutboundServerMessage>,
  registered_players: &mut ResMut<RegisteredPlayers>,
  player_registration_message: &mut MessageWriter<PlayerRegistrationMessage>,
  client_id: ClientId,
  request: SerialisableUnregistrationRequest,
  lobby: &mut ResMut<Lobby>,
  unregisters_local_player: bool,
) {
  if !lobby.validate_registration(&client_id, &request.player_id) {
    warn!(
      "Ignoring invalid unregistration for client [{}] and player [{}]",
      client_id, request.player_id
    );
    return;
  }

  info!("[{}] with client ID [{}] unregistered", request.player_id, client_id);

  if unregisters_local_player {
    utils::unregister_local_player_locally(
      registered_players,
      player_registration_message,
      None,
      request.player_id.into(),
    );
  } else {
    utils::unregister_remote_player_locally(
      registered_players,
      player_registration_message,
      request.player_id.into(),
    );
  }

  lobby.unregister_player(client_id, request.player_id);
  broadcast_player_unregistered(outbound_server_message, client_id, request.player_id.into());
}

/// Processes any incoming messages from clients by applying them locally and broadcasting them to all other clients,
/// if necessary.
fn handle_inbound_client_message(
  mut messages: MessageReader<InboundClientMessage>,
  mut lobby: ResMut<Lobby>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_control_schemes: Res<AvailableControlSchemes>,
  mut player_registration_message: MessageWriter<PlayerRegistrationMessage>,
  mut input_message: MessageWriter<InputMessage>,
  mut outbound_server_message: MessageWriter<OutboundServerMessage>,
) {
  for message in messages.read() {
    match message {
      InboundClientMessage::RegistrationRequest(message, client_id) => {
        handle_registration_request(
          &mut outbound_server_message,
          &mut registered_players,
          &available_control_schemes,
          &mut player_registration_message,
          *client_id,
          ControlSchemeId(message.control_scheme_id),
          &mut lobby,
          false,
        );
      }
      InboundClientMessage::UnregistrationRequest(message, client_id) => {
        handle_unregistration_request(
          &mut outbound_server_message,
          &mut registered_players,
          &mut player_registration_message,
          *client_id,
          *message,
          &mut lobby,
          false,
        );
      }
      InboundClientMessage::Input(message, client_id) => {
        let message: InputMessage = message.into();
        let player_id = match message {
          InputMessage::Action(player_id) => player_id,
          InputMessage::Move(player_id, _) => player_id,
        };
        if lobby.validate_registration(client_id, &player_id.into()) {
          input_message.write(message);
          continue;
        }
        warn!("Received invalid input action on [Unreliable] channel: {:?}", message);
      }
    }
  }
}

/// The main system for server messages.
fn handle_inbound_server_message(
  mut messages: MessageReader<InboundServerMessage>,
  mut outbound_server_message: MessageWriter<OutboundServerMessage>,
  mut lobby: ResMut<Lobby>,
  current_state: Res<State<AppState>>,
  mut next_state: ResMut<NextState<AppState>>,
  seed: Res<Seed>,
  mut registered_players: ResMut<RegisteredPlayers>,
  mut player_registration_message: MessageWriter<PlayerRegistrationMessage>,
) {
  for message in messages.read() {
    match message {
      InboundServerMessage::ClientConnected { client_id } => {
        info!("Client with ID [{}] connected", client_id);

        // TODO: Communicate current state of the lobby (registered players, etc.) to the newly connected client
        let seed_message = encode_to_bytes(&InboundServerMessage::ClientInitialised {
          seed: seed.get(),
          client_id: *client_id,
        })
        .expect("Failed to serialise seed message");
        outbound_server_message.write(OutboundServerMessage::Send {
          client_id: *client_id,
          channel: ChannelType::ReliableOrdered,
          payload: seed_message,
        });

        if *current_state == AppState::Preparing {
          next_state.set(AppState::Initialising);
        }
      }
      InboundServerMessage::ClientDisconnected { client_id } => {
        info!("Client with ID [{}] disconnected", client_id);

        for player_id in lobby.get_registered_players_cloned(client_id) {
          handle_unregistration_request(
            &mut outbound_server_message,
            &mut registered_players,
            &mut player_registration_message,
            *client_id,
            SerialisableUnregistrationRequest { player_id },
            &mut lobby,
            false,
          );
        }
      }
      _ => {}
    }
  }
}

/// Broadcasts the authoritative state (position and rotation) of all snake heads to all clients.
/// This runs every frame to ensure clients have up-to-date positions for interpolation.
fn broadcast_player_states_system(
  mut outbound_server_message: MessageWriter<OutboundServerMessage>,
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

  if let Ok(payload) = encode_to_bytes(&InboundServerMessage::UpdatePlayerStates { states }) {
    outbound_server_message.write(OutboundServerMessage::Broadcast {
      channel: ChannelType::Unreliable,
      payload,
    });
  } else {
    warn!("Failed to serialise player states message");
  }
}

fn handle_local_player_registration_request_message(
  mut messages: MessageReader<LocalPlayerRegistrationRequestMessage>,
  mut lobby: ResMut<Lobby>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_control_schemes: Res<AvailableControlSchemes>,
  mut player_registration_message: MessageWriter<PlayerRegistrationMessage>,
  mut outbound_server_message: MessageWriter<OutboundServerMessage>,
) {
  for request in messages.read() {
    if request.has_registered {
      handle_registration_request(
        &mut outbound_server_message,
        &mut registered_players,
        &available_control_schemes,
        &mut player_registration_message,
        host_client_id(),
        request.control_scheme_id,
        &mut lobby,
        true,
      );
      continue;
    }

    let Some(player_id) = registered_players
      .players
      .iter()
      .find(|player| player.is_local() && player.input.id == request.control_scheme_id)
      .map(|player| player.id)
    else {
      warn!(
        "Ignoring local unregistration for unknown control scheme [{:?}]",
        request.control_scheme_id
      );
      continue;
    };

    handle_unregistration_request(
      &mut outbound_server_message,
      &mut registered_players,
      &mut player_registration_message,
      host_client_id(),
      SerialisableUnregistrationRequest {
        player_id: player_id.into(),
      },
      &mut lobby,
      true,
    );
  }
}

/// A system that handles local state change events and broadcasts them to all connected clients.
fn handle_local_state_transition_event(
  mut messages: MessageReader<StateTransitionEvent<AppState>>,
  mut outbound_server_message: MessageWriter<OutboundServerMessage>,
  winner: Res<WinnerInfo>,
) {
  for message in messages.read() {
    if let Some(state_name) = message.entered {
      let server_event = InboundServerMessage::StateChanged {
        new_state: state_name.to_string(),
        winner_info: winner.get_as_u8(),
      };
      debug!("Broadcasting: {:?}", server_event);
      if let Ok(payload) = encode_to_bytes(&server_event) {
        outbound_server_message.write(OutboundServerMessage::Broadcast {
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

/// A system that processes local exit lobby messages and broadcasts the servers intention to shut down to all connected
/// clients. This will then schedule the actual disconnect after a short delay.
fn process_and_broadcast_local_exit_lobby_message(
  mut messages: MessageReader<ExitLobbyMessage>,
  mut commands: Commands,
  mut outbound_server_message: MessageWriter<OutboundServerMessage>,
) {
  for _ in messages.read() {
    info!("Informing all clients about intention to shut down server and scheduling shutdown...");
    let payload = encode_to_bytes(&InboundServerMessage::ShutdownServer).expect(CLIENT_MESSAGE_SERIALISATION);
    outbound_server_message.write(OutboundServerMessage::Broadcast {
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
  mut outbound_server_message: MessageWriter<OutboundServerMessage>,
) {
  countdown.0.tick(time.delta());
  if countdown.0.just_finished() {
    info!("Disconnecting all clients now...");
    lobby.clear();
    registered_players.clear();
    outbound_server_message.write(OutboundServerMessage::DisconnectAll);
    commands.remove_resource::<ShutdownCountdown>();
    toggle_menu_message.write(ToggleMenuMessage::set(MenuName::MainMenu));
    next_app_state.set(AppState::Preparing);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app_state::AppStatePlugin;
  use crate::prelude::{ControlScheme, SharedMessagesPlugin, SharedResourcesPlugin};
  use bevy::prelude::*;
  use bevy::state::app::StatesPlugin;
  use mooplas_networking::prelude::{
    NetworkingMessagesPlugin, NetworkingResourcesPlugin, SerialisableRegistrationRequest, decode_from_bytes,
  };

  fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins((
      StatesPlugin,
      SharedMessagesPlugin,
      SharedResourcesPlugin,
      NetworkingMessagesPlugin,
      NetworkingResourcesPlugin,
      AppStatePlugin,
    ));
    app
  }

  fn add_control_schemes(app: &mut App, count: u8) {
    let mut available_control_schemes = app
      .world_mut()
      .get_resource_mut::<AvailableControlSchemes>()
      .expect("AvailableControlSchemes resource missing");
    for id in 0..count {
      available_control_schemes.schemes.push(ControlScheme::test(id));
    }
  }

  #[test]
  fn broadcast_player_states_system_sends_state_updates_for_all_snake_heads() {
    let mut app = setup();
    app.add_systems(Update, broadcast_player_states_system);

    app
      .world_mut()
      .spawn((Transform::from_xyz(100.0, 200.0, 0.0), PlayerId(1), SnakeHead));
    app
      .world_mut()
      .spawn((Transform::from_xyz(150.0, 250.0, 0.0), PlayerId(2), SnakeHead));
    app.update();

    let messages = app
      .world_mut()
      .get_resource_mut::<Messages<OutboundServerMessage>>()
      .expect("Messages<OutgoingServerMessage> missing");
    let message_vec: Vec<_> = messages.iter_current_update_messages().collect();
    assert_eq!(message_vec.len(), 1);
    match &message_vec[0] {
      OutboundServerMessage::Broadcast { channel, payload } => {
        assert!(matches!(channel, ChannelType::Unreliable));
        assert!(!payload.is_empty());
      }
      _ => panic!("Expected broadcast message"),
    }
  }

  #[test]
  fn broadcast_player_states_system_does_not_send_when_no_snake_heads() {
    let mut app = setup();
    app.add_systems(Update, broadcast_player_states_system);
    app.update();

    let messages = app
      .world_mut()
      .get_resource_mut::<Messages<OutboundServerMessage>>()
      .expect("Messages<OutgoingServerMessage> missing");
    let message_vec: Vec<_> = messages.iter_current_update_messages().collect();
    assert_eq!(message_vec.len(), 0);
  }

  #[test]
  fn handle_inbound_client_message_assigns_next_available_player_id() {
    let mut app = setup();
    add_control_schemes(&mut app, 3);
    app.add_systems(Update, handle_inbound_client_message);

    {
      let mut registered_players = app.world_mut().resource_mut::<RegisteredPlayers>();
      registered_players
        .register(crate::prelude::RegisteredPlayer::new_mutable(
          PlayerId(0),
          ControlScheme::test(0),
          Color::WHITE,
        ))
        .expect("Host player should register");
    }

    app
      .world_mut()
      .write_message(InboundClientMessage::RegistrationRequest(
        SerialisableRegistrationRequest { control_scheme_id: 0 },
        ClientId::from_renet_u64(42),
      ))
      .expect("Failed to queue InboundClientMessage");
    app.update();

    let registered_players = app.world().resource::<RegisteredPlayers>();
    let remote_player = registered_players
      .players
      .iter()
      .find(|player| player.id == PlayerId(1))
      .expect("Expected a server-assigned player ID");
    assert!(remote_player.is_remote());
    assert_eq!(remote_player.input.id, ControlSchemeId(0));

    let messages = app
      .world_mut()
      .get_resource_mut::<Messages<OutboundServerMessage>>()
      .expect("Messages<OutgoingServerMessage> missing");
    let broadcasts: Vec<_> = messages.iter_current_update_messages().collect();
    assert_eq!(broadcasts.len(), 1);
    match broadcasts[0] {
      OutboundServerMessage::Broadcast { payload, .. } => {
        let decoded: InboundServerMessage = decode_from_bytes(payload).expect("Expected decodable payload");
        assert!(matches!(
          decoded,
          InboundServerMessage::PlayerRegistered {
            client_id,
            player_id: 1,
            control_scheme_id: 0,
          } if client_id == ClientId::from_renet_u64(42)
        ));
      }
      _ => panic!("Expected a broadcast message"),
    }
  }

  #[test]
  fn handle_inbound_client_message_assigns_player_id_above_local_control_scheme_count() {
    let mut app = setup();
    add_control_schemes(&mut app, 5);
    app.add_systems(Update, handle_inbound_client_message);

    {
      let mut registered_players = app.world_mut().resource_mut::<RegisteredPlayers>();
      for id in 0..5 {
        registered_players
          .register(crate::prelude::RegisteredPlayer::new_mutable(
            PlayerId(id),
            ControlScheme::test(id),
            Color::WHITE,
          ))
          .expect("Host player should register");
      }
    }

    app
      .world_mut()
      .write_message(InboundClientMessage::RegistrationRequest(
        SerialisableRegistrationRequest { control_scheme_id: 0 },
        ClientId::from_renet_u64(42),
      ))
      .expect("Failed to queue InboundClientMessage");
    app.update();

    let registered_players = app.world().resource::<RegisteredPlayers>();
    let remote_player = registered_players
      .players
      .iter()
      .find(|player| player.id == PlayerId(5))
      .expect("Expected a sixth player slot to be assigned");
    assert!(remote_player.is_remote());
    assert_eq!(remote_player.input.id, ControlSchemeId(0));
  }

  #[test]
  fn handle_local_player_registration_request_message_registers_local_host_player() {
    let mut app = setup();
    add_control_schemes(&mut app, 2);
    app.add_systems(Update, handle_local_player_registration_request_message);

    app
      .world_mut()
      .write_message(LocalPlayerRegistrationRequestMessage {
        control_scheme_id: ControlSchemeId(1),
        has_registered: true,
      })
      .expect("Failed to queue LocalPlayerRegistrationRequestMessage");
    app.update();

    let registered_players = app.world().resource::<RegisteredPlayers>();
    let local_player = registered_players
      .players
      .iter()
      .find(|player| player.input.id == ControlSchemeId(1))
      .expect("Expected local host player to be registered");
    assert!(local_player.is_local());
    assert_eq!(local_player.id, PlayerId(0));
  }

  #[test]
  fn handle_inbound_client_message_reuses_freed_player_id_after_unregistration() {
    let mut app = setup();
    add_control_schemes(&mut app, 2);
    app.add_systems(Update, handle_inbound_client_message);

    let first_client_id = ClientId::from_renet_u64(42);
    let second_client_id = ClientId::from_renet_u64(43);

    app
      .world_mut()
      .write_message(InboundClientMessage::RegistrationRequest(
        SerialisableRegistrationRequest { control_scheme_id: 0 },
        first_client_id,
      ))
      .expect("Failed to queue initial registration request");
    app.update();

    app
      .world_mut()
      .write_message(InboundClientMessage::UnregistrationRequest(
        SerialisableUnregistrationRequest {
          player_id: PlayerId(0).into(),
        },
        first_client_id,
      ))
      .expect("Failed to queue unregistration request");
    app.update();

    app
      .world_mut()
      .write_message(InboundClientMessage::RegistrationRequest(
        SerialisableRegistrationRequest { control_scheme_id: 1 },
        second_client_id,
      ))
      .expect("Failed to queue replacement registration request");
    app.update();

    let registered_players = app.world().resource::<RegisteredPlayers>();
    let recycled_player = registered_players
      .players
      .iter()
      .find(|player| player.id == PlayerId(0))
      .expect("Expected the freed player ID to be reused");
    assert_eq!(recycled_player.input.id, ControlSchemeId(1));
  }

  #[test]
  fn broadcast_local_app_state_system_broadcasts_state_transitions() {
    let mut app = setup();
    app.add_systems(Update, handle_local_state_transition_event);
    app.update();

    app
      .world_mut()
      .write_message(StateTransitionEvent {
        exited: None,
        entered: Some(AppState::Playing),
        allow_same_state_transitions: false,
      })
      .unwrap();
    app.update();

    let messages = app
      .world_mut()
      .get_resource_mut::<Messages<OutboundServerMessage>>()
      .expect("Messages<OutgoingServerMessage> missing");
    let message_vec: Vec<&OutboundServerMessage> = messages.iter_current_update_messages().collect();
    let playing_messages: Vec<_> = message_vec
      .iter()
      .filter(|message| match message {
        OutboundServerMessage::Broadcast { payload, .. } => {
          let decoded = decode_from_bytes::<InboundServerMessage>(payload);
          matches!(
            decoded,
            Ok(InboundServerMessage::StateChanged { ref new_state, .. }) if new_state == "Playing"
          )
        }
        _ => false,
      })
      .collect();

    assert!(!message_vec.is_empty(), "Expected at least one message");
    assert_eq!(playing_messages.len(), 1, "Expected exactly one state message");
  }

  #[test]
  fn broadcast_local_app_state_system_ignores_transitions_without_entered_state() {
    let mut app = setup();
    app.add_systems(Update, handle_local_state_transition_event);
    app.update();

    let before_count = {
      let messages = app
        .world_mut()
        .get_resource_mut::<Messages<OutboundServerMessage>>()
        .expect("Messages<OutgoingServerMessage> missing");
      messages.iter_current_update_messages().count()
    };

    app
      .world_mut()
      .write_message(StateTransitionEvent::<AppState> {
        exited: Some(AppState::Preparing),
        entered: None,
        allow_same_state_transitions: false,
      })
      .unwrap();
    app.update();

    let after_count = {
      let messages = app
        .world_mut()
        .get_resource_mut::<Messages<OutboundServerMessage>>()
        .expect("Messages<OutgoingServerMessage> missing");
      messages.iter_current_update_messages().count()
    };

    assert_eq!(before_count, after_count);
  }
}
