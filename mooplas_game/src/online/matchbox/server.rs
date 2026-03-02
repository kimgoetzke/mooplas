use crate::app_state::AppState;
use crate::prelude::{AvailablePlayerConfigs, PlayerRegistrationMessage, RegisteredPlayers, Seed};
use bevy::app::{App, Plugin};
use bevy::log::*;
use bevy::prelude::{MessageWriter, NextState, On, Res, ResMut};
use mooplas_networking::prelude::{ChannelType, Lobby, OutgoingServerMessage, ServerEvent, encode_to_bytes};
use mooplas_networking_matchbox::prelude::ServerMatchboxPlugin;

/// A plugin that adds server-side online multiplayer capabilities to the game. Only active when the game is running in
/// server mode. Mutually exclusive with the [`crate::online::renet::ServerPlugin`].
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(ServerMatchboxPlugin)
      .add_observer(receive_server_events);
  }
}

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
        crate::online::shared_server::handle_player_registration_message_from_client(
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
