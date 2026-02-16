use crate::prelude::NetworkRole;
use bevy::prelude::{Component, Event, Message};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

#[cfg(feature = "matchbox")]
use crate::matchbox::RawClientId;

#[cfg(feature = "renet")]
use crate::renet::RawClientId;

/// An enum representing the different types of channels that can be used for sending messages.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChannelType {
  Unreliable,
  ReliableOrdered,
  ReliableUnordered,
}

impl From<ChannelType> for u8 {
  fn from(channel: ChannelType) -> Self {
    match channel {
      ChannelType::Unreliable => 0,
      ChannelType::ReliableUnordered => 1,
      ChannelType::ReliableOrdered => 2,
    }
  }
}

/// A component identifying a player. Used to link player entities together.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg(not(target_arch = "wasm32"))]
pub struct PlayerId(pub u8);

impl Display for PlayerId {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "Player {}", self.0)
  }
}

impl Debug for PlayerId {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "Player {}", self.0)
  }
}

impl Into<u8> for PlayerId {
  fn into(self) -> u8 {
    self.0
  }
}

/// A stable, non-generic client ID wrapper used by messages and APIs. The inner
/// representation varies by target via `RawClientId`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash)]
#[serde(transparent)]
pub struct ClientId(pub RawClientId);

#[cfg(feature = "renet")]
#[cfg(not(feature = "matchbox"))]
impl From<u64> for ClientId {
  fn from(value: u64) -> Self {
    ClientId(value)
  }
}

impl Display for ClientId {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// An event for the client-side code of an application.
#[derive(Event, Debug, Serialize, Deserialize, Component)]
pub enum ServerEvent {
  /// Sent by the server to all clients (except the one that just connected) when a new client has connected.
  ClientConnected { client_id: ClientId },
  /// Sent by the server to all clients (except the one that just disconnected) when a client has disconnected.
  ClientDisconnected { client_id: ClientId },
  /// Sent to a client when they have successfully initialised their connection to the server. Sent by the server in
  /// response to a [`ServerEvent::ClientConnected`] to the client that just connected.
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

/// A message sent by the client. This is how the client communicates to the server. Read by the networking code, not
/// intended to be read by application code.
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
  PlayerRegistration(PlayerRegistrationMessage),
  Input(SerialisableInputMessage),
}

impl ClientMessage {
  pub fn to_event(self, client_id: ClientId) -> ClientEvent {
    match self {
      ClientMessage::PlayerRegistration(message) => ClientEvent::PlayerRegistration(message, client_id),
      ClientMessage::Input(action) => ClientEvent::Input(action, client_id),
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

/// An event for the server-side code of an application. Triggered by the networking code after having received a
/// [`ClientMessage`]. For the consumption of the application code.
#[derive(Event, Serialize, Deserialize)]
pub enum ClientEvent {
  PlayerRegistration(PlayerRegistrationMessage, ClientId),
  Input(SerialisableInputMessage, ClientId),
}

impl Debug for ClientEvent {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      ClientEvent::PlayerRegistration(message, client_id) => {
        write!(
          f,
          "ClientMessage::PlayerRegistration for {} with ID {}",
          message.player_id, client_id
        )
      }
      ClientEvent::Input(action, client_id) => {
        write!(f, "ClientMessage::{:?} for client with ID {}", action, client_id)
      }
    }
  }
}

/// This is how the networking code communicates errors to the application code.
#[derive(Event, Debug)]
pub enum NetworkErrorEvent {
  RenetDisconnect(String),
  NetcodeDisconnect(String),
  NetcodeTransportError(String),
  IoError(String),
  OtherError(String),
}

impl Error for NetworkErrorEvent {}

impl Display for NetworkErrorEvent {
  fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
    Debug::fmt(&self, fmt)
  }
}
