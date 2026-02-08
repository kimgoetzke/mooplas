use serde::Serialize;
use serde::de::DeserializeOwned;

/// Maximum number of bytes allowed to be decoded from an incoming network packet. Just a safety limit.
const MAX_NETWORK_MESSAGE_BYTES: usize = 256 * 1024;

/// Encodes a serialisable value into a vector of bytes using postcard.
pub fn encode_to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, postcard::Error> {
  postcard::to_allocvec(value)
}

/// Decodes a deserialisable value from a slice of bytes using postcard.
pub fn decode_from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, postcard::Error> {
  if bytes.len() > MAX_NETWORK_MESSAGE_BYTES {
    return Err(postcard::Error::DeserializeUnexpectedEnd);
  }
  postcard::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
  use super::*;

  mod online {
    use super::*;
    use crate::prelude::PlayerId;
    use crate::shared::structs::{ClientMessage, NetworkRole, PlayerRegistrationMessage, ServerEvent};

    #[test]
    fn encode_to_bytes_and_decode_from_bytes_client_message_round_trip() {
      let original = ClientMessage::PlayerRegistration(PlayerRegistrationMessage {
        player_id: PlayerId(7),
        has_registered: true,
        is_anyone_registered: true,
        network_role: Some(NetworkRole::Client),
      });
      let bytes = encode_to_bytes(&original).expect("Encode should succeed");
      let decoded: ClientMessage = decode_from_bytes(&bytes).expect("Decode should succeed");
      assert_eq!(format!("{original:?}"), format!("{decoded:?}"));
    }

    #[test]
    fn decode_from_bytes_rejects_garbage_bytes() {
      let bytes = vec![0xde, 0xad, 0xbe, 0xef];
      let decoded: Result<ServerEvent, _> = decode_from_bytes(&bytes);
      assert!(decoded.is_err());
    }
  }
}
