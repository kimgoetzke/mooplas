use crate::prelude::{ChannelType, ClientId, NetworkRole, PlayerId};
use bevy::app::{App, Plugin};
use bevy::prelude::{Component, Message};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Debug, Formatter};

pub struct NetworkingMessagesPlugin;

impl Plugin for NetworkingMessagesPlugin {
  fn build(&self, app: &mut App) {
    app
      .add_message::<PlayerStateUpdateMessage>()
      .add_message::<InboundClientMessage>()
      .add_message::<OutboundClientMessage>()
      .add_message::<InboundServerMessage>()
      .add_message::<OutboundServerMessage>();
  }
}

/// A message containing authoritative state updates for a player from the server. Used for server-to-client state
/// synchronisation.
#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PlayerStateUpdateMessage {
  /// The [`PlayerId`] as a u8
  pub id: u8,
  /// Position (x, y) of the player's snake head
  pub position: (f32, f32),
  /// Rotation in radians around Z axis
  pub rotation: f32,
}

impl PlayerStateUpdateMessage {
  pub fn new(player_id: u8, position: (f32, f32), rotation: f32) -> Self {
    Self {
      id: player_id,
      position,
      rotation,
    }
  }
}

/// A request for the active client transport to send a payload to the server. This is intentionally transport-agnostic.
/// Should be used by application client-side code.
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
pub enum OutboundClientMessage {
  Send { channel: ChannelType, payload: Vec<u8> },
  Disconnect,
}

/// A request for the active server transport to send/broadcast a payload to clients. This is intentionally
/// transport-agnostic. Should be used by application server-side code.
#[derive(Message, Clone, Debug, Serialize, Deserialize)]
pub enum OutboundServerMessage {
  /// Broadcast to all connected clients.
  Broadcast { channel: ChannelType, payload: Vec<u8> },
  /// Broadcast to all connected clients except the provided client.
  BroadcastExcept {
    except_client_id: ClientId,
    channel: ChannelType,
    payload: Vec<u8>,
  },
  /// Send to a specific client.
  Send {
    client_id: ClientId,
    channel: ChannelType,
    payload: Vec<u8>,
  },
  /// Disconnect all connected clients.
  DisconnectAll,
}

/// A message for the client-side code of an application.
#[derive(Message, Debug, Serialize, Deserialize, Component)]
pub enum InboundServerMessage {
  /// Sent by the server to all clients (except the one that just connected) when a new client has connected.
  ClientConnected { client_id: ClientId },
  /// Sent by the server to all clients (except the one that just disconnected) when a client has disconnected.
  ClientDisconnected { client_id: ClientId },
  /// Sent to a client when they have successfully initialised their connection to the server. Sent by the server in
  /// response to a [`InboundServerMessage::ClientConnected`] to the client that just connected.
  ClientInitialised { seed: u64, client_id: ClientId },
  /// Indicates that the app state has changed on the server.
  StateChanged { new_state: String, winner_info: Option<u8> },
  /// Informs clients that a player has registered in the lobby.
  PlayerRegistered { client_id: ClientId, player_id: u8 },
  /// Informs clients that a player has unregistered from the lobby.
  PlayerUnregistered { client_id: ClientId, player_id: u8 },
  /// Contains authoritative player state updates in a vec of (player_id, x, y, rotation).
  UpdatePlayerStates { states: Vec<(u8, f32, f32, f32)> },
  /// Informs the clients that the server is about to shut down. Gives clients time to prepare before being
  /// disconnected.
  ShutdownServer,
}

// A message that is serialisable and communicates an input action.
#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SerialisableInputMessage {
  Move(u8, f32),
  Action(u8),
}

/// A message that communicates a change to a user's registration status in the lobby.
#[derive(Message, Debug, Serialize, Deserialize, Copy, Clone)]
pub struct PlayerRegistrationMessage {
  pub player_id: PlayerId,
  /// Whether the player has registered (true) or unregistered (false).
  pub has_registered: bool,
  /// Whether any player is currently registered, after this change.
  pub is_anyone_registered: bool,
  /// Whether this message originated from the server or the client. Used to prevent echoing.
  pub network_role: Option<NetworkRole>,
}

// TODO: Remove this type and use the message type directly
/// A message sent by the client. This is how the client communicates to the server. Read by the networking code, not
/// intended to be read by application code.
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
  PlayerRegistration(PlayerRegistrationMessage),
  Input(SerialisableInputMessage),
}

impl ClientMessage {
  pub fn to_event(self, client_id: ClientId) -> InboundClientMessage {
    match self {
      ClientMessage::PlayerRegistration(message) => InboundClientMessage::PlayerRegistration(message, client_id),
      ClientMessage::Input(action) => InboundClientMessage::Input(action, client_id),
    }
  }
}

impl Debug for ClientMessage {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      ClientMessage::PlayerRegistration(message) => {
        write!(f, "ClientMessage::PlayerRegistration for {}", message.player_id)
      }
      ClientMessage::Input(action) => {
        write!(f, "ClientMessage::{:?}", action)
      }
    }
  }
}

/// A message for the server-side code of an application. Triggered by the networking code after having received a
/// [`ClientMessage`]. For the consumption of the application code.
#[derive(Message, Serialize, Deserialize)]
pub enum InboundClientMessage {
  PlayerRegistration(PlayerRegistrationMessage, ClientId),
  Input(SerialisableInputMessage, ClientId),
}

impl Debug for InboundClientMessage {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      InboundClientMessage::PlayerRegistration(message, client_id) => {
        write!(
          f,
          "ClientMessage::PlayerRegistration for {} with ID {}",
          message.player_id, client_id
        )
      }
      InboundClientMessage::Input(action, client_id) => {
        write!(f, "ClientMessage::{:?} for client with ID {}", action, client_id)
      }
    }
  }
}
