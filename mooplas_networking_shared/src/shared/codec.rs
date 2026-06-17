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
    use crate::prelude::{ClientId, SerialisableRegisteredPlayer};
    use crate::shared::messages::InboundServerMessage;
    use crate::shared::structs::{ClientMessage, SerialisableRegistrationRequest};

    #[test]
    fn encode_to_bytes_and_decode_from_bytes_client_message_round_trip() {
      let original = ClientMessage::RegistrationRequest(SerialisableRegistrationRequest {
        control_scheme_id: 7,
        name: "Test 1".to_string(),
      });
      let bytes = encode_to_bytes(&original).expect("Encode should succeed");
      let decoded: ClientMessage = decode_from_bytes(&bytes).expect("Decode should succeed");
      assert_eq!(format!("{original:?}"), format!("{decoded:?}"));
    }

    #[test]
    fn decode_from_bytes_rejects_garbage_bytes() {
      let bytes = vec![0xde, 0xad, 0xbe, 0xef];
      let decoded: Result<InboundServerMessage, _> = decode_from_bytes(&bytes);
      assert!(decoded.is_err());
    }

    #[test]
    fn client_initialised_round_trip_carries_seed_without_spawn_points() {
      let original = InboundServerMessage::ClientInitialised {
        seed: 42,
        client_id: ClientId::from_renet_u64(7),
        current_state: "Playing".to_string(),
        registered_players: vec![SerialisableRegisteredPlayer {
          client_id: ClientId::nil(),
          player_id: 0,
          control_scheme_id: 0,
          name: "Host".to_string(),
        }],
        winner_info: Some(0),
      };
      let bytes = encode_to_bytes(&original).expect("Encode should succeed");
      let decoded: InboundServerMessage = decode_from_bytes(&bytes).expect("Decode should succeed");

      let InboundServerMessage::ClientInitialised {
        seed,
        client_id,
        current_state,
        registered_players,
        winner_info,
      } = decoded
      else {
        panic!("Expected ClientInitialised");
      };
      assert_eq!(seed, 42);
      assert_eq!(client_id, ClientId::from_renet_u64(7));
      assert_eq!(current_state, "Playing");
      assert_eq!(registered_players.len(), 1);
      assert_eq!(winner_info, Some(0));
    }
  }
}
