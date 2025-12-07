use crate::online::lib::messages::SerialisableInputAction;
use crate::prelude::PlayerRegistrationMessage;
use bevy::prelude::Component;
use bevy_renet::renet::ClientId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessage {
  ClientConnected { client_id: ClientId },
  ClientDisconnected { client_id: ClientId },
  ClientInitialised { seed: u64, client_id: ClientId },
  StateChanged { new_state: String },
  PlayerRegistered { client_id: ClientId, player_id: u8 },
  PlayerUnregistered { client_id: ClientId, player_id: u8 },
}

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
  PlayerRegistrationMessage(PlayerRegistrationMessage),
  InputAction(u32, SerialisableInputAction),
}

impl Debug for ClientMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ClientMessage::PlayerRegistrationMessage(message) => {
        write!(f, "ClientMessage::PlayerRegistrationMessage for {}", message.player_id)
      }
      ClientMessage::InputAction(sequence, action) => {
        write!(f, "ClientMessage::{:?} (#{})", action, sequence)
      }
    }
  }
}
