use crate::prelude::{InboundClientMessage, NetworkRole};
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

#[cfg(feature = "renet")]
impl From<bevy_renet::renet::DefaultChannel> for ChannelType {
  fn from(value: bevy_renet::renet::DefaultChannel) -> Self {
    match value {
      bevy_renet::renet::DefaultChannel::Unreliable => ChannelType::Unreliable,
      bevy_renet::renet::DefaultChannel::ReliableOrdered => ChannelType::ReliableOrdered,
      bevy_renet::renet::DefaultChannel::ReliableUnordered => ChannelType::ReliableUnordered,
    }
  }
}

#[cfg(feature = "renet")]
impl From<ChannelType> for bevy_renet::renet::DefaultChannel {
  fn from(value: ChannelType) -> Self {
    match value {
      ChannelType::Unreliable => bevy_renet::renet::DefaultChannel::Unreliable,
      ChannelType::ReliableOrdered => bevy_renet::renet::DefaultChannel::ReliableOrdered,
      ChannelType::ReliableUnordered => bevy_renet::renet::DefaultChannel::ReliableUnordered,
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

impl Into<u8> for PlayerId {
  fn into(self) -> u8 {
    self.0
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

  /// Creates a deterministic, stable UUID for a renet u64 client ID.
  pub fn from_renet_u64(value: u64) -> Self {
    let mut bytes = [0_u8; 16];
    bytes[8..].copy_from_slice(&value.to_be_bytes());
    Self(Uuid::from_bytes(bytes))
  }

  /// Extracts a renet u64 client ID from the backing UUID.
  pub fn to_renet_u64(self) -> u64 {
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

/// A type that communicates a change to a user's registration status in the lobby.
#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct SerialisablePlayerRegistration {
  pub player_id: PlayerId,
  /// Whether the player has registered (true) or unregistered (false).
  pub has_registered: bool,
  /// Whether any player is currently registered, after this change.
  pub is_anyone_registered: bool,
  /// Whether this message originated from the server or the client. Used to prevent echoing.
  pub network_role: Option<NetworkRole>,
}

/// The type sent by the networking code of the client. It's the same as [`InboundClientMessage`] but doesn't contain
/// the [`ClientId`] of the sender. This is how the client communicates to the server. Deserialised by the networking
/// code, not intended to be used by any application code. Converted to [`InboundClientMessage`] and then made
/// available to the application code.
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
  PlayerRegistration(SerialisablePlayerRegistration),
  Input(SerialisableInput),
}

impl ClientMessage {
  pub fn to_inbound_message(self, client_id: ClientId) -> InboundClientMessage {
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
