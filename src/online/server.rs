use crate::app_states::AppState;
use crate::online::lib::{Lobby, OnlinePlayer, ServerMessages, utils};
use crate::prelude::{
  AvailablePlayerConfigs, ContinueMessage, NetworkAudience, PlayerId, PlayerRegistrationMessage, RegisteredPlayers,
  Seed, has_registered_players,
};
use bevy::app::{App, Plugin, Update};
use bevy::log::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::{
  IntoScheduleConfigs, MessageReader, MessageWriter, NextState, Query, Res, ResMut, Transform, in_state,
  resource_exists,
};
use bevy_renet::RenetServerPlugin;
use bevy_renet::netcode::NetcodeServerPlugin;
use bevy_renet::renet::{ClientId, DefaultChannel, RenetServer, ServerEvent};

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins((RenetServerPlugin, NetcodeServerPlugin))
      .add_systems(
        Update,
        (
          handle_server_event_messages,
          handle_client_updates_system,
          server_sync_players_system,
        )
          .run_if(resource_exists::<RenetServer>),
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
        handle_own_updates_system
          .run_if(in_state(AppState::Registering))
          .run_if(resource_exists::<RenetServer>),
      );
  }
}

fn server_sync_players_system(mut server: ResMut<RenetServer>, query: Query<(&Transform, &OnlinePlayer)>) {
  let mut lobby: HashMap<ClientId, [f32; 3]> = HashMap::new();
  for (transform, player) in query.iter() {
    lobby.insert(player.id, transform.translation.into());
  }

  let sync_message = bincode::serialize(&lobby).expect("Failed to serialize sync message");
  server.broadcast_message(DefaultChannel::Unreliable, sync_message);
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
        let connected_message = bincode::serialize(&ServerMessages::ClientConnected { client_id: *client_id })
          .expect("Failed to serialise client message");
        server.broadcast_message_except(*client_id, DefaultChannel::ReliableOrdered, connected_message);
        lobby.connected.push(*client_id);

        // Send the current seed to the newly connected client
        let seed_message = bincode::serialize(&ServerMessages::SeedSynchronised { seed: seed.get() })
          .expect("Failed to serialise seed message");
        server.send_message(*client_id, DefaultChannel::ReliableOrdered, seed_message);
      }
      ServerEvent::ClientDisconnected { client_id, reason } => {
        info!("Client with ID [{}] disconnected: {}", client_id, reason);

        // Notify all other clients about the disconnection
        let message = bincode::serialize(&ServerMessages::ClientDisconnected { client_id: *client_id })
          .expect("Failed to serialise client message");
        server.broadcast_message_except(*client_id, DefaultChannel::ReliableOrdered, message);
        lobby.connected.retain(|&id| id != *client_id);
      }
    }

    // TODO: Improve state transition logic
    if lobby.connected.len() > 0 {
      next_state.set(AppState::Initialising);
      let message = bincode::serialize(&ServerMessages::StateChanged {
        new_state: AppState::Initialising.to_string(),
      })
      .expect("Failed to serialise client message");
      server.broadcast_message(DefaultChannel::ReliableOrdered, message);
    }
  }
}

/// Processes any incoming messages from clients by applying them locally and broadcasting them to all other clients.
fn handle_client_updates_system(
  lobby: Res<Lobby>,
  mut server: ResMut<RenetServer>,
  mut registered_players: ResMut<RegisteredPlayers>,
  available_configs: Res<AvailablePlayerConfigs>,
  mut player_registration_message_writer: MessageWriter<PlayerRegistrationMessage>,
) {
  for client_id in lobby.connected.iter() {
    while let Some(message) = server.receive_message(*client_id, DefaultChannel::ReliableOrdered) {
      let client_message = bincode::deserialize(&message).expect("Failed to deserialise client message");
      match client_message {
        PlayerRegistrationMessage {
          player_id,
          has_registered,
          ..
        } => {
          handle_player_registration_message_from_client(
            &mut server,
            &mut registered_players,
            &available_configs,
            &mut player_registration_message_writer,
            client_id,
            player_id,
            has_registered,
          );
        }
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
    let message = bincode::serialize(&ServerMessages::PlayerRegistered {
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
    let message = bincode::serialize(&ServerMessages::PlayerUnregistered {
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

fn handle_own_updates_system(mut messages: MessageReader<PlayerRegistrationMessage>, mut server: ResMut<RenetServer>) {
  for message in messages.read() {
    if utils::should_message_be_skipped(&message, NetworkAudience::Client) {
      continue;
    }
    if message.has_registered {
      debug!(
        "Informing all clients that [Player {}] registered locally...",
        message.player_id.0
      );
      let message = bincode::serialize(&ServerMessages::PlayerRegistered {
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
      let message = bincode::serialize(&ServerMessages::PlayerUnregistered {
        client_id: 0,
        player_id: message.player_id.0,
      })
      .expect("Failed to serialise client message");
      server.broadcast_message(DefaultChannel::ReliableOrdered, message);
    }
  }
}

fn handle_continue_message(mut continue_messages: MessageReader<ContinueMessage>, mut server: ResMut<RenetServer>) {
  let messages = continue_messages.read().collect::<Vec<&ContinueMessage>>();
  if messages.is_empty() {
    return;
  }
  let state_changed_message_for_clients = bincode::serialize(&ServerMessages::StateChanged {
    new_state: AppState::Playing.to_string(),
  })
  .expect("Failed to serialise client message");
  server.broadcast_message(DefaultChannel::ReliableOrdered, state_changed_message_for_clients);
}
