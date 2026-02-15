#![cfg(feature = "wasm")]

use crate::prelude::{ClientMessage, ServerEvent};
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::js_sys::{Array, ArrayBuffer, Uint8Array};
use web_sys::{MessageEvent, RtcConfiguration, RtcDataChannelInit, RtcIceServer, RtcPeerConnection};

#[derive(Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
  Client(ClientMessage),
  Server(ServerEvent),
}

pub struct NetworkChannel {
  pub incoming: Receiver<NetworkMessage>,
  pub outgoing: Sender<NetworkMessage>,
}

pub fn create_peer(incoming_tx: Sender<NetworkMessage>, outgoing_rx: Receiver<NetworkMessage>) -> RtcPeerConnection {
  // ICE configuration with public STUN server
  let ice_server = RtcIceServer::new();
  ice_server.set_urls(&JsValue::from_str("stun:stun.l.google.com:19302"));
  let config = RtcConfiguration::new();
  config.set_ice_servers(&Array::of1(&ice_server));
  let peer_connection = RtcPeerConnection::new_with_configuration(&config).unwrap();

  // Create data channel
  // https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/createDataChannel
  let data_chanel_init = RtcDataChannelInit::new();
  data_chanel_init.set_ordered(true);
  let data_channel = peer_connection.create_data_channel_with_data_channel_dict("mooplas_game", &data_chanel_init);

  // Incoming messages (WebRTC → Bevy)
  let incoming_tx = incoming_tx.clone();
  let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
    if let Ok(buf) = e.data().dyn_into::<ArrayBuffer>() {
      let bytes = Uint8Array::new(&buf).to_vec();
      if let Ok(msg) = crate::prelude::decode_from_bytes(&bytes) {
        let _ = incoming_tx.send(msg);
      }
    }
  });

  data_channel.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
  on_message.forget();

  // Outgoing messages (Bevy → WebRTC)
  let data_channel = Rc::new(data_channel);
  wasm_bindgen_futures::spawn_local(async move {
    while let Ok(msg) = outgoing_rx.recv() {
      let bytes = crate::prelude::encode_to_bytes(&msg).unwrap();
      let _ = data_channel.send_with_u8_array(&bytes);
    }
  });

  peer_connection
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::{ClientId, RawClientId, ServerEvent};
  use crossbeam_channel::unbounded;

  #[test]
  fn can_sent_network_message_over_network_channel() {
    let (to_tx, to_rx) = unbounded();
    let (_, from_rx) = unbounded();
    let channel = NetworkChannel {
      incoming: from_rx,
      outgoing: to_tx,
    };
    let expected_client_id = ClientId(7.0 as RawClientId);
    assert!(
      channel
        .outgoing
        .send(NetworkMessage::Server(ServerEvent::ClientConnected {
          client_id: expected_client_id
        }))
        .is_ok()
    );
    assert!(!to_rx.is_empty());
    match to_rx.recv().unwrap() {
      NetworkMessage::Server(ServerEvent::ClientConnected { client_id }) => {
        assert_eq!(client_id, expected_client_id);
      }
      _ => panic!("Unexpected message type"),
    }
  }
}
