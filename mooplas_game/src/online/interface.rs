use crate::prelude::{InputMessage, NetworkRole, PlayerId, PlayerRegistrationMessage};
use bevy::prelude::{App, Plugin};
use mooplas_networking::prelude::SerialisableInputActionMessage;

/// A plugin that acts as an interface between local and online functionalities.
pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
  fn build(&self, _: &mut App) {}
}

impl From<&PlayerId> for mooplas_networking::prelude::PlayerId {
  fn from(value: &PlayerId) -> Self {
    mooplas_networking::prelude::PlayerId(value.0)
  }
}

impl From<PlayerId> for mooplas_networking::prelude::PlayerId {
  fn from(value: PlayerId) -> Self {
    mooplas_networking::prelude::PlayerId(value.0)
  }
}

impl Into<PlayerId> for mooplas_networking::prelude::PlayerId {
  fn into(self) -> PlayerId {
    PlayerId(self.0)
  }
}

impl From<&PlayerRegistrationMessage> for mooplas_networking::prelude::PlayerRegistrationMessage {
  fn from(value: &PlayerRegistrationMessage) -> Self {
    mooplas_networking::prelude::PlayerRegistrationMessage {
      player_id: value.player_id.into(),
      has_registered: value.has_registered,
      is_anyone_registered: value.is_anyone_registered,
      network_role: value.network_role.map(|role| match role {
        NetworkRole::None => mooplas_networking::prelude::NetworkRole::None,
        NetworkRole::Server => mooplas_networking::prelude::NetworkRole::Server,
        NetworkRole::Client => mooplas_networking::prelude::NetworkRole::Client,
      }),
    }
  }
}

impl Into<PlayerRegistrationMessage> for mooplas_networking::prelude::PlayerRegistrationMessage {
  fn into(self) -> PlayerRegistrationMessage {
    PlayerRegistrationMessage {
      player_id: self.player_id.into(),
      has_registered: self.has_registered,
      is_anyone_registered: self.is_anyone_registered,
      network_role: self.network_role.map(|role| match role {
        mooplas_networking::prelude::NetworkRole::None => NetworkRole::None,
        mooplas_networking::prelude::NetworkRole::Server => NetworkRole::Server,
        mooplas_networking::prelude::NetworkRole::Client => NetworkRole::Client,
      }),
    }
  }
}

impl From<&InputMessage> for SerialisableInputActionMessage {
  fn from(value: &InputMessage) -> Self {
    match value {
      InputMessage::Move(player_id, direction) => SerialisableInputActionMessage::Move(player_id.0, *direction),
      InputMessage::Action(player_id) => SerialisableInputActionMessage::Action(player_id.0),
    }
  }
}

impl Into<InputMessage> for SerialisableInputActionMessage {
  fn into(self) -> InputMessage {
    match self {
      SerialisableInputActionMessage::Move(player_id, direction) => InputMessage::Move(PlayerId(player_id), direction),
      SerialisableInputActionMessage::Action(player_id) => InputMessage::Action(PlayerId(player_id)),
    }
  }
}
