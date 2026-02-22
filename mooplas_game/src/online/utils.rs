#![cfg(feature = "online")]

use crate::prelude::{AvailablePlayerConfigs, PlayerId, PlayerRegistrationMessage, RegisteredPlayers};
use crate::shared::RegisteredPlayer;
use bevy::log::*;
use bevy::prelude::{MessageWriter, Res, ResMut};
use mooplas_networking::prelude::NetworkRole;

pub(crate) fn register_player_locally(
  registered_players: &mut ResMut<RegisteredPlayers>,
  available_configs: &Res<AvailablePlayerConfigs>,
  player_registration_message: &mut MessageWriter<PlayerRegistrationMessage>,
  player_id: PlayerId,
) {
  let config = available_configs
    .find_by_id(player_id)
    .expect("Failed to find player config for registered player");
  match registered_players.register(RegisteredPlayer::new_immutable_from(config)) {
    Ok(_) => {
      info!("[{}] has registered (remotely)", player_id);
      player_registration_message.write(PlayerRegistrationMessage {
        player_id,
        has_registered: true,
        is_anyone_registered: true,
        network_role: None,
      });
    }
    Err(e) => warn!("Failed to register [{}]: {}", player_id, e),
  }
}

pub(crate) fn unregister_player_locally(
  registered_players: &mut ResMut<RegisteredPlayers>,
  messages: &mut MessageWriter<PlayerRegistrationMessage>,
  player_id: PlayerId,
) {
  match registered_players.unregister_immutable(player_id) {
    Ok(_) => {
      info!("[{}] has unregistered (remotely)", player_id);
      messages.write(PlayerRegistrationMessage {
        player_id,
        has_registered: false,
        is_anyone_registered: registered_players.count() > 0,
        network_role: None,
      });
    }
    Err(e) => warn!("[{}] was not registered: {}", player_id, e),
  }
}

pub(crate) fn should_message_be_skipped(
  message: &PlayerRegistrationMessage,
  network_role_to_skip: NetworkRole,
) -> bool {
  if match &message.network_role {
    Some(network_role) => network_role == &network_role_to_skip,
    None => true,
  } {
    return true;
  }
  false
}
