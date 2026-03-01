use bevy::prelude::Commands;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::{PeerId, WebRtcSocket};
use bevy_matchbox::prelude::ChannelConfig;
use mooplas_networking::prelude::{ClientId, ClientNetworkingActive, ServerNetworkingActive};
use rand::RngExt;
use rand::distr::Alphanumeric;

const ROOM_NAME_LENGTH: usize = 8;

/// Generates a WebSocket room URL with a random UUID as the room name.
pub fn generate_room_url() -> String {
  let room_id: String = rand::rng()
    .sample_iter(&Alphanumeric)
    .take(ROOM_NAME_LENGTH)
    .map(char::from)
    .collect();
  format!("ws://localhost:3536/{}", room_id)
}

/// Validates that a string looks like a valid WebSocket URL. Checks:
/// - Starts with `ws://` or `wss://`
/// - Contains a port number (e.g., `:3536`)
/// - Contains a path with a room identifier (e.g., `/room-id`)
///
/// Returns `Ok(())` if valid, or Err with a message if invalid.
pub fn validate_websocket_url(url: &str) -> Result<(), String> {
  // Check protocol
  if !url.starts_with("ws://") && !url.starts_with("wss://") {
    return Err("URL must start with ws:// or wss://".to_string());
  }
  if !url.contains(':') || url.matches(':').count() < 2 {
    return Err("URL must include a port number (e.g., :3536)".to_string());
  }
  let parts: Vec<&str> = url.split('/').collect();
  if parts.len() < 4 {
    return Err("URL must include a room identifier path (e.g., /room-id)".to_string());
  }
  let room_id = parts[3];
  if room_id.is_empty() {
    return Err("Room identifier cannot be empty".to_string());
  }

  Ok(())
}

// TODO: Stop room ID from being entirely ignored by the server
pub fn start_socket(commands: &mut Commands, room_url: &str) -> Result<(), String> {
  validate_websocket_url(room_url)?;
  let web_rtc_socket_builder = WebRtcSocket::builder(room_url)
    .add_unreliable_channel()
    .add_reliable_channel()
    .add_channel(ChannelConfig {
      ordered: false,
      max_retransmits: None,
    });
  let socket = MatchboxSocket::from(web_rtc_socket_builder);
  commands.insert_resource(socket);
  Ok(())
}

/// Give it a [`PeerId`] from Matchbox, it converts it to a [`ClientId`] used by the game.
pub fn client_id_from_peer_id(peer_id: PeerId) -> ClientId {
  ClientId::from_uuid(peer_id.0)
}

/// Give it a [`ClientId`] from the game, it converts it to a [`PeerId`] used by Matchbox.
pub fn peer_id_from_client_id(client_id: ClientId) -> PeerId {
  PeerId(client_id.as_uuid())
}

/// Cleans up all networking resources for native platforms.
pub fn remove_all_matchbox_resources(commands: &mut Commands) {
  commands.remove_resource::<ClientNetworkingActive>();
  commands.remove_resource::<ServerNetworkingActive>();
  commands.remove_resource::<MatchboxSocket>();
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generate_room_url_returns_valid_format() {
    let url = generate_room_url();
    let parts: Vec<&str> = url.split('/').collect();
    let room_id = parts[3];
    assert!(url.starts_with("ws://localhost:3536/"));
    assert_eq!(parts.len(), 4); // ["ws:", "", "localhost:3536", "<uuid>"]
    assert!(!room_id.is_empty());
    assert_eq!(room_id.len(), ROOM_NAME_LENGTH);
  }

  #[test]
  fn generate_room_url_generates_unique_urls() {
    let url1 = generate_room_url();
    let url2 = generate_room_url();
    assert_ne!(url1, url2, "Should generate unique room URLs");
  }

  #[test]
  fn validate_websocket_url_accepts_valid_ws_url() {
    let result = validate_websocket_url("ws://localhost:3536/room-123");
    assert!(result.is_ok());
  }

  #[test]
  fn validate_websocket_url_accepts_valid_wss_url() {
    let result = validate_websocket_url("wss://example.com:3536/room-456");
    assert!(result.is_ok());
  }

  #[test]
  fn validate_websocket_url_rejects_non_websocket_protocol() {
    let result = validate_websocket_url("http://localhost:3536/room");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("ws://"));
  }

  #[test]
  fn validate_websocket_url_rejects_url_without_port() {
    let result = validate_websocket_url("ws://localhost/room");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("port"));
  }

  #[test]
  fn validate_websocket_url_rejects_url_without_path() {
    let result = validate_websocket_url("ws://localhost:3536");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("room"));
  }

  #[test]
  fn validate_websocket_url_rejects_url_with_only_slash() {
    let result = validate_websocket_url("ws://localhost:3536/");
    assert!(result.is_err());
  }
}
