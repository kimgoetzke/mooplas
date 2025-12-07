use crate::app_states::AppState;
use crate::online::lib::{ClientMessage, InputSequence, Lobby, SerialisableInputAction, ServerMessage, utils};
use crate::prelude::{
  AvailablePlayerConfigs, ContinueMessage, InputAction, NetworkAudience, PlayerId, PlayerRegistrationMessage,
  RegisteredPlayers, Seed, has_registered_players,
};
use bevy::app::{App, Plugin, Update};
use bevy::log::*;
use bevy::prelude::{
  IntoScheduleConfigs, MessageReader, MessageWriter, NextState, Res, ResMut, in_state, resource_exists,
};
use bevy_renet::RenetServerPlugin;
use bevy_renet::netcode::NetcodeServerPlugin;
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer, ServerEvent};

/// A plugin that adds server-side online multiplayer capabilities to the game. Only active when the game is running in
/// server mode. Mutually exclusive with the [`crate::online::ClientPlugin`].
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((RenetServerPlugin, NetcodeServerPlugin))
      .add_systems(
        Update,
        (handle_server_event_messages, handle_ordered_client_messages_system).run_if(resource_exists::<RenetServer>),
      )
      .add_systems(
        Update,
        handle_continue_message
          .run_if(in_state(AppState::Registering))
          .run_if(has_registered_players)
          .run_if(resource_exists::<RenetServer>),
      )
      .add_systems(
        Update,
        handle_local_player_registration_message
          .run_if(in_state(AppState::Registering))
          .run_if(resource_exists::<RenetServer>),
      )
      .add_systems(
        Update,
        (
          handle_unreliable_client_player_movements_system,
          handle_local_input_action_messages,
        )
          .run_if(in_state(AppState::Playing))
          .run_if(resource_exists::<RenetServer>),
      );
  }
}

fn handle_server_event_messages(
  mut messages: MessageReader<ServerEvent>,
  mut server: ResMut<RenetServer>,
  mut lobby: ResMut<Lobby>,
  mut next_state: ResMut<NextState<AppState>>,
  seed: Res<Seed>,
) {
  for message in messages.read() {
    match message {
      ServerEvent::ClientConnected { client_id } => {
        info!("Client with ID [{}] connected", client_id);

        // Notify all other clients about the new connection
        let connected_message = bincode::serialize(&ServerMessage::ClientConnected { client_id: *client_id })
          .expect("Failed to serialise client message");
        server.broadcast_message_except(*client_id, DefaultChannel::ReliableOrdered, connected_message);
        lobby.connected.push(*client_id);

        // Send the current seed to the newly connected client
        let seed_message = bincode::serialize(&ServerMessage::ClientInitialised {
          seed: seed.get(),
          client_id: *client_id,
        })
        .expect("Failed to serialise seed message");
        server.send_message(*client_id, DefaultChannel::ReliableOrdered, seed_message);
      }
      ServerEvent::ClientDisconnected { client_id, reason } => {
        info!("Client with ID [{}] disconnected: {}", client_id, reason);

        // Notify all other clients about the disconnection
        let message = bincode::serialize(&ServerMessage::ClientDisconnected { client_id: *client_id })
          .expect("Failed to serialise client message");
        server.broadcast_message_except(*client_id, DefaultChannel::ReliableOrdered, message);
        lobby.connected.retain(|&id| id != *client_id);
      }
    }

    // TODO: Improve state transition logic
    if lobby.connected.len() > 0 {
      next_state.set(AppState::Initialising);
      let message = bincode::serialize(&ServerMessage::StateChanged {
        new_state: AppState::Initialising.to_string(),
      })
      .expect("Failed to serialise client message");
      server.broadcast_message(DefaultChannel::ReliableOrdered, message);
    }
  }
}

/// Processes any incoming [`DefaultChannel::Unreliable`] messages from clients by applying them locally and
/// broadcasting them to all other clients.
fn handle_unreliable_client_player_movements_system(
  lobby: Res<Lobby>,
  mut server: ResMut<RenetServer>,
  mut input_action_message: MessageWriter<InputAction>,
) {
  for client_id in lobby.connected.iter() {
    while let Some(message) = server.receive_message(*client_id, DefaultChannel::Unreliable) {
      if let Ok(client_message) = bincode::deserialize(&message) {
        match client_message {
          ClientMessage::InputAction(_, action) => {
            input_action_message.write(action.into());
            server.broadcast_message_except(*client_id, DefaultChannel::Unreliable, message);
          }
          _ => {
            warn!(
              "Received unrecognised client message on [Unreliable] channel from client ID [{}]: {:?}",
              client_id, client_message
            );
          }
        }
      } else {
        warn!(
          "Failed to deserialise client message on [Unreliable] channel from client ID [{}]",
          client_id
        );
      }
    }
  }
}

/// A system that handles local input action messages for mutable players byt sends them to the server in order to sync
/// the movements of the local player(s) with the server.
fn handle_local_input_action_messages(
  mut messages: MessageReader<SerialisableInputAction>,
  registered_players: Res<RegisteredPlayers>,
  mut server: ResMut<RenetServer>,
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
        server.broadcast_message(DefaultChannel::Unreliable, input_message);
      } else {
        warn!("Failed to serialise input action message: {:?}", message);
      }
    }
  }
}

/// Processes any incoming [`DefaultChannel::ReliableOrdered`] messages from clients by applying them locally and
/// broadcasting them to all other clients.
fn handle_ordered_client_messages_system(
  lobby: Res<Lobby>,
  mut server: ResMut<RenetServer>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut player_registration_message_writer: MessageWriter<PlayerRegistrationMessage>,
) {
  for client_id in lobby.connected.iter() {
    while let Some(message) = server.receive_message(*client_id, DefaultChannel::ReliableOrdered) {
      if let Ok(client_message) = bincode::deserialize(&message) {
        match client_message {
          ClientMessage::PlayerRegistrationMessage(player_registration_message) => {
            handle_player_registration_message_from_client(
              &mut server,
              &mut registered_players,
              &available_configs,
              &mut player_registration_message_writer,
              client_id,
              player_registration_message.player_id,
              player_registration_message.has_registered,
            );
          }
          _ => {
            warn!(
              "Received unrecognised client message on [ReliableOrdered] channel from client ID [{}]: {:?}",
              client_id, client_message
            );
          }
        }
      } else {
        warn!(
          "Failed to deserialise client message on [ReliableOrdered] channel from client ID [{}]",
          client_id
        );
      }
    }
  }
}

/// Processes an individual player registration message from [`ClientId`].
fn handle_player_registration_message_from_client(
  server: &mut ResMut<RenetServer>,
  mut registered_players: &mut ResMut<RegisteredPlayers>,
  available_configs: &Res<AvailablePlayerConfigs>,
  mut player_registration_message_writer: &mut MessageWriter<PlayerRegistrationMessage>,
  client_id: &ClientId,
  player_id: PlayerId,
  has_registered: bool,
) {
  if has_registered {
    info!("[{}] with client ID [{}] registered", player_id, client_id);
    let message = bincode::serialize(&ServerMessage::PlayerRegistered {
      client_id: *client_id,
      player_id: player_id.0,
    })
    .expect("Failed to serialise client message");
    server.broadcast_message_except(*client_id, DefaultChannel::ReliableOrdered, message);
    utils::register_player_locally(
      &mut registered_players,
      &available_configs,
      &mut player_registration_message_writer,
      player_id,
      None,
    );
  } else {
    info!("[{}] with client ID [{}] unregistered", player_id, client_id);
    let message = bincode::serialize(&ServerMessage::PlayerUnregistered {
      client_id: *client_id,
      player_id: player_id.0,
    })
    .expect("Failed to serialise client message");
    server.broadcast_message_except(*client_id, DefaultChannel::ReliableOrdered, message);
    utils::unregister_player_locally(
      &mut registered_players,
      &mut player_registration_message_writer,
      player_id,
      None,
    );
  }
}

/// A system that handles local messages (such as player registration messages) and broadcasts them to all connected
/// clients.
fn handle_local_player_registration_message(
  mut messages: MessageReader<PlayerRegistrationMessage>,
  mut server: ResMut<RenetServer>,
) {
  for message in messages.read() {
    if utils::should_message_be_skipped(&message, NetworkAudience::Client) {
      continue;
    }
    if message.has_registered {
      debug!(
        "Informing all clients that [Player {}] registered locally...",
        message.player_id.0
      );
      let message = bincode::serialize(&ServerMessage::PlayerRegistered {
        client_id: 0,
        player_id: message.player_id.0,
      })
      .expect("Failed to serialise client message");
      server.broadcast_message(DefaultChannel::ReliableOrdered, message);
    } else {
      debug!(
        "Informing all clients that [Player {}] unregistered locally...",
        message.player_id.0
      );
      let message = bincode::serialize(&ServerMessage::PlayerUnregistered {
        client_id: 0,
        player_id: message.player_id.0,
      })
      .expect("Failed to serialise client message");
      server.broadcast_message(DefaultChannel::ReliableOrdered, message);
    }
  }
}

/// Handles local [`ContinueMessage`] messages and broadcasts a state change to all connected clients. Only the server
/// is permitted to change the game state.
fn handle_continue_message(mut continue_messages: MessageReader<ContinueMessage>, mut server: ResMut<RenetServer>) {
  let messages = continue_messages.read().collect::<Vec<&ContinueMessage>>();
  if messages.is_empty() {
    return;
  }
  let message = bincode::serialize(&ServerMessage::StateChanged {
    new_state: AppState::Playing.to_string(),
  })
  .expect("Failed to serialise client message");
  server.broadcast_message(DefaultChannel::ReliableOrdered, message);
}
