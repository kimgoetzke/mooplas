use bevy::prelude::Commands;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::{PeerId, RtcIceServerConfig, WebRtcSocket};
use bevy_matchbox::prelude::ChannelConfig;
use mooplas_networking::prelude::{ClientId, ClientNetworkingActive, ServerNetworkingActive};
use rand::RngExt;
use rand::distr::Alphanumeric;
use url::Url;

const ROOM_NAME_LENGTH: usize = 8;
const STUN_ICE_SERVER_URL: &str = "stun:stun.l.google.com:19302";

/// Generates a WebSocket room URL with a random room identifier appended to the given signalling server base URL.
pub fn generate_room_url(signalling_server_base_url: &str) -> String {
  let room_id: String = rand::rng()
    .sample_iter(&Alphanumeric)
    .take(ROOM_NAME_LENGTH)
    .map(char::from)
    .collect();
  format!("{}/{}", signalling_server_base_url.trim_end_matches('/'), room_id)
}

pub fn room_id_from_room_url(room_url: &str) -> Result<String, String> {
  validate_websocket_url(room_url)?;
  let parsed_url = Url::parse(room_url).map_err(|error| format!("URL is not valid: {error}"))?;
  let room_id = parsed_url
    .path_segments()
    .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
    .ok_or("URL must include a room identifier path (e.g., /room-id)".to_string())?;
  Ok(room_id.to_string())
}

/// Give it the signalling server base URL and a connection string (either a full room URL or just a room ID) and it
/// resolves it to a full room URL.
///
/// Examples:
/// - Base URL `wss://signal.example.com` and connection string `room-456` will resolve to:
///   `wss://signal.example.com/room-456`.
/// - Base URL `ws://localhost:3536` and connection string `wss://signal.example.com/room-456` will resolve to:
///   `wss://signal.example.com/room-456` (i.e. the connection string remains unchanged).
pub fn resolve_room_url(signalling_server_base_url: &str, connection_string: &str) -> Result<String, String> {
  let connection_string = connection_string.trim();
  if connection_string.is_empty() {
    return Err("Room ID or websocket URL cannot be empty".to_string());
  }
  if connection_string.contains("://") {
    validate_websocket_url(connection_string)?;
    return Ok(connection_string.to_string());
  }
  let room_id = connection_string.trim_matches('/');
  if room_id.is_empty() {
    return Err("Room ID cannot be empty".to_string());
  }
  if room_id.contains('/') {
    return Err("Room ID must not contain '/'".to_string());
  }
  let room_url = format!("{}/{}", signalling_server_base_url.trim_end_matches('/'), room_id);
  validate_websocket_url(&room_url)?;
  Ok(room_url)
}

/// Validates that a string looks like a valid WebSocket URL. Checks:
/// - Starts with `ws://` or `wss://`
/// - Contains a port number for `ws://` URLs (e.g., `:3536`)
/// - Contains a path with a room identifier (e.g., `/room-id`)
///
/// Returns `Ok(())` if valid, or Err with a message if invalid.
pub fn validate_websocket_url(url: &str) -> Result<(), String> {
  let parsed_url = Url::parse(url).map_err(|error| format!("URL is not valid: {error}"))?;
  if !matches!(parsed_url.scheme(), "ws" | "wss") {
    return Err("URL must start with ws:// or wss://".to_string());
  }
  if parsed_url.host_str().is_none() {
    return Err("URL must include a host".to_string());
  }
  if parsed_url.scheme() == "ws" && parsed_url.port().is_none() {
    return Err("URL must include a port number (e.g., :3536)".to_string());
  }
  if parsed_url.path().trim_matches('/').is_empty() {
    return Err("URL must include a room identifier path (e.g., /room-id)".to_string());
  }
  Ok(())
}

// TODO: Stop room ID from being entirely ignored by the server
pub fn start_socket(commands: &mut Commands, room_url: &str) -> Result<(), String> {
  validate_websocket_url(room_url)?;
  let web_rtc_socket_builder = WebRtcSocket::builder(room_url)
    .ice_server(signalling_ice_server_config())
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

pub fn signalling_ice_server_config() -> RtcIceServerConfig {
  RtcIceServerConfig {
    urls: vec![STUN_ICE_SERVER_URL.to_string()],
    username: None,
    credential: None,
  }
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
    let url = generate_room_url("wss://signal.example.com");
    let parts: Vec<&str> = url.split('/').collect();
    let room_id = parts[3];
    assert!(url.starts_with("wss://signal.example.com/"));
    assert_eq!(parts.len(), 4); // ["wss:", "", "signal.example.com", "<room-id>"]
    assert!(!room_id.is_empty());
    assert_eq!(room_id.len(), ROOM_NAME_LENGTH);
  }

  #[test]
  fn generate_room_url_generates_unique_urls() {
    let url1 = generate_room_url("ws://localhost:3536");
    let url2 = generate_room_url("ws://localhost:3536");
    assert_ne!(url1, url2, "Should generate unique room URLs");
  }

  #[test]
  fn generate_room_url_strips_trailing_slash_from_base_url() {
    let url = generate_room_url("wss://signal.example.com/");
    assert!(url.starts_with("wss://signal.example.com/"));
    let url_suffix = url.trim_start_matches("wss://");
    assert!(!url_suffix.contains("//"));
  }

  #[test]
  fn room_id_from_room_url_returns_room_id() {
    let room_id = room_id_from_room_url("wss://signal.example.com/room-456")
      .expect("Expected a room ID to be extracted from a valid room URL");

    assert_eq!(room_id, "room-456");
  }

  #[test]
  fn resolve_room_url_appends_room_id_to_signalling_server_url() {
    let room_url = resolve_room_url("wss://signal.example.com", "room-456")
      .expect("Expected a room ID to resolve against the signalling server URL");

    assert_eq!(room_url, "wss://signal.example.com/room-456");
  }

  #[test]
  fn resolve_room_url_accepts_full_room_url() {
    let room_url = resolve_room_url("ws://localhost:3536", "wss://signal.example.com/room-456")
      .expect("Expected a full room URL to be accepted unchanged");

    assert_eq!(room_url, "wss://signal.example.com/room-456");
  }

  #[test]
  fn resolve_room_url_rejects_non_websocket_url_input() {
    let error = resolve_room_url("wss://signal.example.com", "https://example.com/room-456")
      .expect_err("Expected a non-websocket URL to be rejected");

    assert!(error.contains("ws://"));
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
  fn validate_websocket_url_accepts_valid_wss_url_without_port() {
    let result = validate_websocket_url("wss://example.com/room-456");
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

  #[test]
  fn signalling_ice_server_config_uses_public_google_stun_server() {
    let config = signalling_ice_server_config();
    assert_eq!(config.urls, vec!["stun:stun.l.google.com:19302".to_string()]);
    assert_eq!(config.username, None);
    assert_eq!(config.credential, None);
  }
}
