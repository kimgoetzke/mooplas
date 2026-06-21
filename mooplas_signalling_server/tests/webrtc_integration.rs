use std::{net::SocketAddr, sync::Once, time::Duration};

use matchbox_socket::{Error as MatchboxError, PeerId, PeerState, RtcIceServerConfig, WebRtcSocket};
use mooplas_signalling_server::{ServerConfig, build_server, error::ServerError};
use tokio::task::JoinHandle;

static CRYPTO_PROVIDER: Once = Once::new();

struct MatchboxPeer {
  socket: WebRtcSocket,
  message_loop: JoinHandle<Result<(), MatchboxError>>,
}

fn install_crypto_provider() {
  CRYPTO_PROVIDER.call_once(|| {
    let _ = rustls::crypto::ring::default_provider().install_default();
  });
}

async fn spawn_server() -> (SocketAddr, JoinHandle<Result<(), ServerError>>) {
  let mut server = build_server(ServerConfig { port: 0, tls: None }).expect("Failed to build signalling server");
  let socket_addr = server.bind().expect("Failed to bind signalling server");
  let server_handle = tokio::spawn(server.serve());
  (socket_addr, server_handle)
}

fn spawn_matchbox_peer(socket_addr: SocketAddr, path: &str) -> MatchboxPeer {
  let (socket, message_loop) = WebRtcSocket::builder(format!("ws://{socket_addr}{path}"))
    .reconnect_attempts(Some(1))
    .ice_server(RtcIceServerConfig {
      urls: Vec::new(),
      username: None,
      credential: None,
    })
    .add_reliable_channel()
    .build();
  let message_loop = tokio::spawn(message_loop);
  MatchboxPeer { socket, message_loop }
}

async fn wait_for_id(peer: &mut MatchboxPeer) -> PeerId {
  tokio::time::timeout(Duration::from_secs(5), async {
    loop {
      if let Some(peer_id) = peer.socket.id() {
        return peer_id;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
  })
  .await
  .expect("Timed out waiting for Matchbox peer ID")
}

async fn wait_for_connection(peer: &mut MatchboxPeer, expected_peer: PeerId) {
  tokio::time::timeout(Duration::from_secs(10), async {
    loop {
      let changes = peer
        .socket
        .try_update_peers()
        .expect("Socket message loop ended before peer connection established");
      if changes
        .iter()
        .any(|(peer_id, state)| *peer_id == expected_peer && *state == PeerState::Connected)
        || peer.socket.connected_peers().any(|peer_id| peer_id == expected_peer)
      {
        return;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
  })
  .await
  .expect("Timed out waiting for WebRTC peer connection")
}

async fn wait_for_packet(peer: &mut MatchboxPeer, expected_sender: PeerId, expected_packet: &[u8]) {
  tokio::time::timeout(Duration::from_secs(5), async {
    loop {
      let messages = peer.socket.channel_mut(0).receive();
      if messages
        .iter()
        .any(|(sender, packet)| *sender == expected_sender && packet.as_ref() == expected_packet)
      {
        return;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
  })
  .await
  .expect("Timed out waiting for WebRTC data-channel packet")
}

fn connected_peers(peer: &mut MatchboxPeer) -> Vec<PeerId> {
  peer.socket.update_peers();
  peer.socket.connected_peers().collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_rooms_establish_independent_matchbox_webrtc_connections() {
  install_crypto_provider();
  let (socket_addr, server_handle) = spawn_server().await;

  let mut host_a = spawn_matchbox_peer(socket_addr, "/room-a?role=host");
  let mut host_b = spawn_matchbox_peer(socket_addr, "/room-b?role=host");
  let host_a_id = wait_for_id(&mut host_a).await;
  let host_b_id = wait_for_id(&mut host_b).await;

  let mut client_a = spawn_matchbox_peer(socket_addr, "/room-a");
  let mut client_b = spawn_matchbox_peer(socket_addr, "/room-b");
  let client_a_id = wait_for_id(&mut client_a).await;
  let client_b_id = wait_for_id(&mut client_b).await;

  wait_for_connection(&mut host_a, client_a_id).await;
  wait_for_connection(&mut client_a, host_a_id).await;
  wait_for_connection(&mut host_b, client_b_id).await;
  wait_for_connection(&mut client_b, host_b_id).await;

  assert_eq!(connected_peers(&mut host_a), vec![client_a_id]);
  assert_eq!(connected_peers(&mut client_a), vec![host_a_id]);
  assert_eq!(connected_peers(&mut host_b), vec![client_b_id]);
  assert_eq!(connected_peers(&mut client_b), vec![host_b_id]);

  client_a
    .socket
    .channel_mut(0)
    .send(b"room-a".to_vec().into_boxed_slice(), host_a_id);
  client_b
    .socket
    .channel_mut(0)
    .send(b"room-b".to_vec().into_boxed_slice(), host_b_id);

  wait_for_packet(&mut host_a, client_a_id, b"room-a").await;
  wait_for_packet(&mut host_b, client_b_id, b"room-b").await;

  host_a.message_loop.abort();
  host_b.message_loop.abort();
  client_a.message_loop.abort();
  client_b.message_loop.abort();
  server_handle.abort();
}

#[tokio::test]
async fn duplicate_host_fails_at_matchbox_socket_level() {
  install_crypto_provider();
  let (socket_addr, server_handle) = spawn_server().await;

  let mut host = spawn_matchbox_peer(socket_addr, "/duplicate-room?role=host");
  let _host_id = wait_for_id(&mut host).await;

  let duplicate = spawn_matchbox_peer(socket_addr, "/duplicate-room?role=host");
  let duplicate_result = tokio::time::timeout(Duration::from_secs(2), duplicate.message_loop)
    .await
    .expect("Timed out waiting for duplicate host socket to fail")
    .expect("Duplicate host task panicked");

  assert!(
    duplicate_result.is_err(),
    "Duplicate host message loop should fail, got {duplicate_result:?}"
  );

  host.message_loop.abort();
  server_handle.abort();
}
