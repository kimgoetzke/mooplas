use std::{str::FromStr, time::Duration};

use futures::{SinkExt, StreamExt};
use matchbox_protocol::{JsonPeerEvent, JsonPeerRequest, PeerId, PeerRequest};
use mooplas_signalling_server::{ServerConfig, build_server, error::ServerError};
use tokio::net::TcpStream;
use tokio_tungstenite::{
  MaybeTlsStream, WebSocketStream, connect_async,
  tungstenite::{Error as TungsteniteError, Message},
};

async fn spawn_server() -> (std::net::SocketAddr, tokio::task::JoinHandle<Result<(), ServerError>>) {
  let mut server = build_server(ServerConfig { port: 0, tls: None }).expect("Failed to build signalling server");
  let socket_addr = server.bind().expect("Failed to bind signalling server");
  let server_handle = tokio::spawn(server.serve());
  (socket_addr, server_handle)
}

async fn connect_peer(
  socket_addr: std::net::SocketAddr,
  path: &str,
) -> (WebSocketStream<MaybeTlsStream<TcpStream>>, PeerId) {
  let (mut socket, _) = connect_async(format!("ws://{socket_addr}{path}"))
    .await
    .expect("Failed to connect websocket peer");
  let peer_id = match read_event(&mut socket).await {
    JsonPeerEvent::IdAssigned(peer_id) => peer_id,
    event => panic!("Expected ID assignment, got {event:?}"),
  };
  (socket, peer_id)
}

async fn read_event(socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> JsonPeerEvent {
  let message = tokio::time::timeout(Duration::from_secs(1), socket.next())
    .await
    .expect("Peer event timed out")
    .expect("Websocket closed")
    .expect("Failed to read websocket message");
  let Message::Text(text) = message else {
    panic!("Expected text message, got {message:?}");
  };
  JsonPeerEvent::from_str(&text).expect("Failed to parse peer event")
}

async fn assert_no_event(socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) {
  let event = tokio::time::timeout(Duration::from_millis(75), socket.next()).await;
  assert!(event.is_err(), "Expected no peer event, got {event:?}");
}

async fn send_signal(
  socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
  receiver: PeerId,
  data: serde_json::Value,
) {
  let request: JsonPeerRequest = PeerRequest::Signal { receiver, data };
  socket
    .send(Message::Text(request.to_string().into()))
    .await
    .expect("Failed to signal request");
}

async fn assert_connect_rejected(socket_addr: std::net::SocketAddr, path: &str, expected_status: u16) {
  let error = connect_async(format!("ws://{socket_addr}{path}"))
    .await
    .expect_err("Websocket connection should be rejected");
  match error {
    TungsteniteError::Http(response) => assert_eq!(response.status().as_u16(), expected_status),
    error => panic!("Expected HTTP rejection, got {error:?}"),
  }
}

#[tokio::test]
async fn host_and_client_connect_in_one_room() {
  let (socket_addr, server_handle) = spawn_server().await;
  let (mut host, _) = connect_peer(socket_addr, "/room-a?role=host").await;
  let (_, client_id) = connect_peer(socket_addr, "/room-a").await;

  assert_eq!(read_event(&mut host).await, JsonPeerEvent::NewPeer(client_id));

  server_handle.abort();
  let _ = server_handle.await;
}

#[tokio::test]
async fn duplicate_host_invalid_role_and_hostless_client_are_rejected() {
  let (socket_addr, server_handle) = spawn_server().await;
  let (_host, _) = connect_peer(socket_addr, "/room-a?role=host").await;

  assert_connect_rejected(socket_addr, "/room-a?role=host", 409).await;
  assert_connect_rejected(socket_addr, "/room-a?role=spectator", 400).await;
  assert_connect_rejected(socket_addr, "/room-b", 409).await;

  server_handle.abort();
  let _ = server_handle.await;
}

#[tokio::test]
async fn rooms_are_isolated_on_one_standalone_server() {
  let (socket_addr, server_handle) = spawn_server().await;
  let (mut host_a, _) = connect_peer(socket_addr, "/room-a?role=host").await;
  let (mut host_b, _) = connect_peer(socket_addr, "/room-b?role=host").await;
  let (_client_a, client_a_id) = connect_peer(socket_addr, "/room-a").await;
  let (_client_b, client_b_id) = connect_peer(socket_addr, "/room-b").await;

  assert_eq!(read_event(&mut host_a).await, JsonPeerEvent::NewPeer(client_a_id));
  assert_eq!(read_event(&mut host_b).await, JsonPeerEvent::NewPeer(client_b_id));
  assert_no_event(&mut host_a).await;
  assert_no_event(&mut host_b).await;

  server_handle.abort();
  let _ = server_handle.await;
}

#[tokio::test]
async fn signals_are_routed_only_inside_their_room() {
  let (socket_addr, server_handle) = spawn_server().await;
  let (mut host_a, host_a_id) = connect_peer(socket_addr, "/room-a?role=host").await;
  let (mut host_b, _) = connect_peer(socket_addr, "/room-b?role=host").await;
  let (mut client_a, client_a_id) = connect_peer(socket_addr, "/room-a").await;
  let (mut client_b, client_b_id) = connect_peer(socket_addr, "/room-b").await;
  assert_eq!(read_event(&mut host_a).await, JsonPeerEvent::NewPeer(client_a_id));
  assert_eq!(read_event(&mut host_b).await, JsonPeerEvent::NewPeer(client_b_id));

  send_signal(&mut client_a, host_a_id, serde_json::json!("client-a-to-host-a")).await;
  assert_eq!(
    read_event(&mut host_a).await,
    JsonPeerEvent::Signal {
      sender: client_a_id,
      data: serde_json::json!("client-a-to-host-a"),
    }
  );
  assert_no_event(&mut host_b).await;

  send_signal(&mut host_a, client_a_id, serde_json::json!("host-a-to-client-a")).await;
  assert_eq!(
    read_event(&mut client_a).await,
    JsonPeerEvent::Signal {
      sender: host_a_id,
      data: serde_json::json!("host-a-to-client-a"),
    }
  );
  assert_no_event(&mut client_b).await;

  send_signal(&mut host_a, client_b_id, serde_json::json!("cross-room")).await;
  assert_no_event(&mut client_b).await;

  server_handle.abort();
  let _ = server_handle.await;
}

#[tokio::test]
async fn disconnects_only_affect_their_room() {
  let (socket_addr, server_handle) = spawn_server().await;
  let (mut host_a, _) = connect_peer(socket_addr, "/room-a?role=host").await;
  let (mut host_b, host_b_id) = connect_peer(socket_addr, "/room-b?role=host").await;
  let (mut client_a, client_a_id) = connect_peer(socket_addr, "/room-a").await;
  let (mut client_b, client_b_id) = connect_peer(socket_addr, "/room-b").await;
  assert_eq!(read_event(&mut host_a).await, JsonPeerEvent::NewPeer(client_a_id));
  assert_eq!(read_event(&mut host_b).await, JsonPeerEvent::NewPeer(client_b_id));

  client_a.close(None).await.expect("Failed to close client A");
  assert_eq!(read_event(&mut host_a).await, JsonPeerEvent::PeerLeft(client_a_id));
  assert_no_event(&mut host_b).await;

  host_a.close(None).await.expect("Failed to close host A");
  assert_no_event(&mut client_b).await;

  let (mut client_b_2, client_b_2_id) = connect_peer(socket_addr, "/room-b").await;
  assert_eq!(read_event(&mut host_b).await, JsonPeerEvent::NewPeer(client_b_2_id));

  host_b.close(None).await.expect("Failed to close host B");
  assert_eq!(read_event(&mut client_b).await, JsonPeerEvent::PeerLeft(host_b_id));
  assert_eq!(read_event(&mut client_b_2).await, JsonPeerEvent::PeerLeft(host_b_id));

  server_handle.abort();
  let _ = server_handle.await;
}
