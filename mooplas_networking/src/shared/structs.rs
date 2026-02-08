use bevy::prelude::{Component, Event, Message, Resource};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

pub enum NetworkingType {
  Wasm,
  Native,
}

#[cfg(target_arch = "wasm32")]
pub type RawClientId = u64;

#[cfg(not(target_arch = "wasm32"))]
pub type RawClientId = bevy_renet::renet::ClientId;

/// A component identifying a player. Used to link player entities together.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg(not(target_arch = "wasm32"))]
pub struct PlayerId(pub u8);

impl Display for PlayerId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Player {}", self.0)
  }
}

impl Debug for PlayerId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash)]
#[serde(transparent)]
pub struct ClientId(pub RawClientId);

impl From<u64> for ClientId {
  fn from(value: u64) -> Self {
    ClientId(value)
  }
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessage {
  /// Sent by the server to all clients (except the one that just connected) when a new client has connected.
  ClientConnected { client_id: ClientId },
  /// Sent by the server to all clients (except the one that just disconnected) when a client has disconnected.
  ClientDisconnected { client_id: ClientId },
  /// Sent to a client when they have successfully initialised their connection to the server. Sent by the server in
  /// response to a [`ServerMessage::ClientConnected`] to the client that just connected.
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

// Local serialisable input action message so this crate can compile independently.
// Matches the shape used elsewhere in the project.
#[derive(Message, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SerialisableInputActionMessage {
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

/// A resource that indicates the current network role of this application instance. Only relevant in online
/// multiplayer mode.
#[derive(Resource, Debug, PartialEq, Eq, Clone, Copy, Default, Serialize, Deserialize)]
pub enum NetworkRole {
  #[default]
  None,
  Server,
  Client,
}

// ClientMessage is generic over M (the player registration payload). M must implement
// serde-serialisable/deserialisable; we express that via serde(bound). We avoid placing
// additional trait bounds (e.g., Message) on the enum definition itself to keep this
// crate flexible — callers who require `Message` can add that bound where they use
// the enum. This also avoids lifetime shadowing issues with serde derive.
#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
  PlayerRegistration(PlayerRegistrationMessage),
  Input(SerialisableInputActionMessage),
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

#[derive(Event, Debug)]
pub enum MooplasNetworkingErrorEvent {
  RenetDisconnect(String),
  NetcodeDisconnect(String),
  NetcodeTransportError(String),
  IoError(String),
  Other(String),
}

impl Error for MooplasNetworkingErrorEvent {}

impl Display for MooplasNetworkingErrorEvent {
  fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
    Debug::fmt(&self, fmt)
  }
}
