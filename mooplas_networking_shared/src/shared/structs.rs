use crate::prelude::InboundClientMessage;
use bevy::prelude::{Component, Event};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

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

impl From<ChannelType> for usize {
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

impl From<PlayerId> for u8 {
  fn from(value: PlayerId) -> u8 {
    value.0
  }
}

/// A stable, transport-agnostic client ID wrapper used by messages and APIs.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash)]
#[serde(transparent)]
pub struct ClientId(Uuid);

impl ClientId {
  /// Creates a [`ClientId`] from a UUID.
  pub fn from_uuid(value: Uuid) -> Self {
    Self(value)
  }

  /// Returns the UUID backing this [`ClientId`].
  pub fn as_uuid(&self) -> Uuid {
    self.0
  }

  /// Creates a deterministic, stable UUID for a u64 client ID.
  pub fn from_u64(value: u64) -> Self {
    let mut bytes = [0_u8; 16];
    bytes[8..].copy_from_slice(&value.to_be_bytes());
    Self(Uuid::from_bytes(bytes))
  }

  /// Extracts a u64 client ID from the backing UUID.
  pub fn to_u64(self) -> u64 {
    let bytes = self.0.as_bytes();
    u64::from_be_bytes(bytes[8..].try_into().expect("Expected 8 bytes"))
  }

  /// Returns a nil/zero UUID client ID. Useful for tests and defaults.
  pub fn nil() -> Self {
    Self(Uuid::from_u128(0))
  }
}

impl From<Uuid> for ClientId {
  fn from(value: Uuid) -> Self {
    Self::from_uuid(value)
  }
}

impl Display for ClientId {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// This is how the networking code communicates errors to the application code.
#[derive(Event, Debug)]
pub enum NetworkErrorEvent {
  Disconnect(String),
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

// A type that is serialisable and communicates an input action.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SerialisableInput {
  Move(u8, f32),
  Action(u8),
}

/// A type that communicates a local control scheme registration request.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SerialisableRegistrationRequest {
  pub control_scheme_id: u8,
  pub name: String,
}

/// A type that communicates an unregistration request for an authoritative player identity.
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct SerialisableUnregistrationRequest {
  pub player_id: PlayerId,
}

/// A player registration included in the authoritative client bootstrap.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SerialisableRegisteredPlayer {
  pub client_id: ClientId,
  pub player_id: u8,
  pub control_scheme_id: u8,
  pub name: String,
}

/// A player in an online game. Only used by the [`prelude::Lobby`] resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInLobby {
  pub player_id: PlayerId,
  pub control_scheme_id: u8,
}

/// The type sent by the networking code of the client. It's the same as [`InboundClientMessage`] but doesn't contain
/// the [`ClientId`] of the sender. This is how the client communicates to the server. Deserialised by the networking
/// code, not intended to be used by any application code. Converted to [`InboundClientMessage`] and then made
/// available to the application code.
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
  RegistrationRequest(SerialisableRegistrationRequest),
  UnregistrationRequest(SerialisableUnregistrationRequest),
  Input(SerialisableInput),
}

impl ClientMessage {
  pub fn to_inbound_message(self, client_id: ClientId) -> InboundClientMessage {
    match self {
      ClientMessage::RegistrationRequest(message) => InboundClientMessage::RegistrationRequest(message, client_id),
      ClientMessage::UnregistrationRequest(message) => InboundClientMessage::UnregistrationRequest(message, client_id),
      ClientMessage::Input(action) => InboundClientMessage::Input(action, client_id),
    }
  }
}

impl Debug for ClientMessage {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      ClientMessage::RegistrationRequest(message) => {
        write!(
          f,
          "ClientMessage::RegistrationRequest for control scheme {}",
          message.control_scheme_id
        )
      }
      ClientMessage::UnregistrationRequest(message) => {
        write!(f, "ClientMessage::UnregistrationRequest for {}", message.player_id)
      }
      ClientMessage::Input(action) => {
        write!(f, "ClientMessage::{:?}", action)
      }
    }
  }
}
