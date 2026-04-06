use crate::online::structs::LocalInputMapping;
use crate::prelude::{
  AvailableControlSchemes, ControlSchemeId, PlayerId, PlayerRegistrationMessage, RegisteredPlayers,
  colour_for_player_id,
};
use crate::shared::RegisteredPlayer;
use bevy::log::*;
use bevy::prelude::{MessageWriter, Res, ResMut};

pub(crate) fn register_remote_player_locally(
  registered_players: &mut ResMut<RegisteredPlayers>,
  available_control_schemes: &Res<AvailableControlSchemes>,
  player_registration_message: &mut MessageWriter<PlayerRegistrationMessage>,
  player_id: PlayerId,
  control_scheme_id: ControlSchemeId,
) {
  let control_scheme = control_scheme_for_id(available_control_schemes, control_scheme_id);
  let colour = colour_for_player_id(player_id);
  match registered_players.register(RegisteredPlayer::new_immutable(player_id, control_scheme, colour)) {
    Ok(()) => {
      info!("[{}] has registered (remotely)", player_id);
      player_registration_message.write(PlayerRegistrationMessage {
        player_id,
        control_scheme_id: None,
        is_anyone_registered: true,
      });
    }
    Err(error) => warn!("Failed to register [{}]: {}", player_id, error),
  }
}

pub(crate) fn register_local_player_locally(
  registered_players: &mut ResMut<RegisteredPlayers>,
  available_control_schemes: &Res<AvailableControlSchemes>,
  player_registration_message: &mut MessageWriter<PlayerRegistrationMessage>,
  local_input_mapping: Option<&mut ResMut<LocalInputMapping>>,
  player_id: PlayerId,
  control_scheme_id: ControlSchemeId,
) {
  let control_scheme = control_scheme_for_id(available_control_schemes, control_scheme_id);
  let colour = colour_for_player_id(player_id);
  match registered_players.register(RegisteredPlayer::new_mutable(player_id, control_scheme, colour)) {
    Ok(()) => {
      info!("[{}] has registered (locally)", player_id);
      if let Some(local_input_mapping) = local_input_mapping {
        local_input_mapping.insert(control_scheme_id, player_id);
      }
      player_registration_message.write(PlayerRegistrationMessage {
        player_id,
        control_scheme_id: Some(control_scheme_id),
        is_anyone_registered: true,
      });
    }
    Err(error) => warn!("Failed to register [{}]: {}", player_id, error),
  }
}

fn control_scheme_for_id(
  available_control_schemes: &Res<AvailableControlSchemes>,
  control_scheme_id: ControlSchemeId,
) -> crate::prelude::ControlScheme {
  available_control_schemes
    .find_by_id(control_scheme_id)
    .unwrap_or_else(|| {
      panic!(
        "Failed to find control scheme [{:?}] for registered player",
        control_scheme_id
      )
    })
    .clone()
}

pub(crate) fn unregister_remote_player_locally(
  registered_players: &mut ResMut<RegisteredPlayers>,
  messages: &mut MessageWriter<PlayerRegistrationMessage>,
  player_id: PlayerId,
) {
  match registered_players.unregister_immutable(player_id) {
    Ok(()) => {
      info!("[{}] has unregistered (remotely)", player_id);
      messages.write(PlayerRegistrationMessage {
        player_id,
        control_scheme_id: None,
        is_anyone_registered: registered_players.count() > 0,
      });
    }
    Err(error) => warn!("[{}] was not registered: {}", player_id, error),
  }
}

pub(crate) fn unregister_local_player_locally(
  registered_players: &mut ResMut<RegisteredPlayers>,
  messages: &mut MessageWriter<PlayerRegistrationMessage>,
  local_input_mapping: Option<&mut ResMut<LocalInputMapping>>,
  player_id: PlayerId,
) {
  let control_scheme_id = registered_players
    .players
    .iter()
    .find(|player| player.id == player_id && player.is_local())
    .map(|player| player.input.id);

  match registered_players.unregister_mutable(player_id) {
    Ok(()) => {
      info!("[{}] has unregistered (locally)", player_id);
      if let Some(control_scheme_id) = control_scheme_id
        && let Some(local_input_mapping) = local_input_mapping
      {
        local_input_mapping.remove(&control_scheme_id);
      }
      messages.write(PlayerRegistrationMessage {
        player_id,
        control_scheme_id,
        is_anyone_registered: registered_players.count() > 0,
      });
    }
    Err(error) => warn!("[{}] was not registered: {}", player_id, error),
  }
}
