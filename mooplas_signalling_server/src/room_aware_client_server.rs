use std::{
  collections::{HashMap, VecDeque},
  net::SocketAddr,
};

use async_trait::async_trait;
use axum::{
  extract::ws::Message,
  http::StatusCode,
  response::{IntoResponse, Response},
};
use futures::StreamExt;
use matchbox_protocol::{JsonPeerEvent, PeerId, PeerRequest};
use matchbox_signaling::{
  Callback, ClientRequestError, SignalingCallbacks, SignalingServerBuilder, SignalingState, SignalingTopology,
  WsStateMeta,
  common_logic::{SignalingChannel, StateObj, parse_request, try_send},
};
use tracing::{error, info, warn};

/// Marker for the room-aware client-server topology.
#[derive(Debug, Default)]
pub(crate) struct RoomAwareClientServer;

#[expect(
  clippy::result_large_err,
  reason = "matchbox_signaling requires axum::response::Response for connection rejection"
)]
/// Creates a signalling server builder with room-aware client-server routing. This is required because Matchbox only
/// supports rooms for full mesh topologies, not client-server.
pub(crate) fn room_aware_client_server_builder(
  socket_addr: impl Into<SocketAddr>,
) -> SignalingServerBuilder<RoomAwareClientServer, RoomAwareClientServerCallbacks, RoomAwareClientServerState> {
  let state = RoomAwareClientServerState::default();
  let request_state = state.clone();
  SignalingServerBuilder::new(socket_addr, RoomAwareClientServer, state).on_connection_request(move |connection| {
    info!("Connecting: {connection:?}...");
    let room = connection.path.unwrap_or_else(|| "world".to_string());
    let role = parse_role(&connection.query_params)?;

    match role {
      PeerRole::Host if request_state.has_host_or_pending_host(&room) => {
        Err((StatusCode::CONFLICT, "Room already has a host\n").into_response())
      }
      PeerRole::Client if !request_state.has_host_or_pending_host(&room) => {
        Err((StatusCode::CONFLICT, "Room has no host\n").into_response())
      }
      role => {
        request_state.approve_role(&room, role);
        Ok(true)
      }
    }
  })
}

#[async_trait]
impl SignalingTopology<RoomAwareClientServerCallbacks, RoomAwareClientServerState> for RoomAwareClientServer {
  /// Handles one WebSocket connection until it closes.
  async fn state_machine(metastate: WsStateMeta<RoomAwareClientServerCallbacks, RoomAwareClientServerState>) {
    let WsStateMeta {
      room,
      peer_id,
      sender,
      mut receiver,
      mut state,
      callbacks,
    } = metastate;

    let Some(role) = state.take_pending_role(&room) else {
      warn!("No approved role found for peer [{peer_id}] in room [{room}]");
      return;
    };

    match role {
      PeerRole::Host => {
        if !state.add_host(&room, peer_id, sender.clone()) {
          warn!("Rejected duplicate host [{peer_id}] for room [{room}] after upgrade");
          return;
        }
        info!("Host joined and has ID [{peer_id}]");
        callbacks.host_connected.emit(peer_id);
      }
      PeerRole::Client => match state.add_client(&room, peer_id, sender.clone()) {
        Ok(()) => {
          info!("Client with ID [{peer_id}] connected");
          callbacks.client_connected.emit(peer_id);
        }
        Err(error) => {
          error!("Failed to connect client [{peer_id}] to room [{room}]: {error:?}");
          return;
        }
      },
    }

    while let Some(request) = receiver.next().await {
      let request = match parse_request(request) {
        Ok(request) => request,
        Err(error) => {
          log_client_request_error(peer_id, &error);
          state.disconnect_peer(&room, peer_id, role, &callbacks);
          return;
        }
      };

      match request {
        PeerRequest::Signal { receiver, data } => {
          let event = Message::Text(JsonPeerEvent::Signal { sender: peer_id, data }.to_string().into());
          let result = match role {
            PeerRole::Host => state.try_send_to_client(&room, receiver, event),
            PeerRole::Client => state.try_send_to_host(&room, event),
          };
          if let Err(error) = result {
            error!("Error sending signal event in room [{room}]: {error:?}");
          }
        }
        PeerRequest::KeepAlive => {}
      }
    }

    state.disconnect_peer(&room, peer_id, role, &callbacks);
  }
}

/// Lifecycle callbacks for room-aware client-server signalling. Required by Matchbox's topology trait but unused in
/// practice at this point.
#[derive(Default, Debug, Clone)]
pub(crate) struct RoomAwareClientServerCallbacks {
  client_connected: Callback<PeerId>,
  client_disconnected: Callback<PeerId>,
  host_connected: Callback<PeerId>,
  host_disconnected: Callback<PeerId>,
}

impl SignalingCallbacks for RoomAwareClientServerCallbacks {}

/// Shared mutable signalling state for all rooms.
#[derive(Debug, Default, Clone)]
pub(crate) struct RoomAwareClientServerState {
  state: StateObj<RoomsState>,
}

impl SignalingState for RoomAwareClientServerState {}

/// Registry of active rooms, peers, and approved role handshakes.
#[derive(Debug, Default, Clone)]
struct RoomsState {
  rooms: HashMap<String, RoomState>,
  peers: HashMap<PeerId, PeerRoomMembership>,
  pending_roles: HashMap<String, VecDeque<PeerRole>>,
}

/// Host and client channels for one room.
#[derive(Debug, Default, Clone)]
struct RoomState {
  host: Option<(PeerId, SignalingChannel)>,
  clients: HashMap<PeerId, SignalingChannel>,
}

/// Tracks which room and role a peer owns.
#[derive(Debug, Clone)]
struct PeerRoomMembership {
  room: String,
  role: PeerRole,
}

/// Distinguishes host sockets from client sockets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeerRole {
  Host,
  Client,
}

const LOCK_ERROR: &str = "Room state mutex is poisoned";

impl RoomAwareClientServerState {
  /// Returns whether a room has, or is about to have, a host.
  fn has_host_or_pending_host(&self, room: &str) -> bool {
    let state = self.state.lock().expect(LOCK_ERROR);
    state.rooms.get(room).and_then(|room| room.host.as_ref()).is_some()
      || state
        .pending_roles
        .get(room)
        .is_some_and(|roles| roles.iter().any(|role| matches!(role, PeerRole::Host)))
  }

  /// Stores the approved role for the upcoming WebSocket upgrade.
  fn approve_role(&self, room: &str, role: PeerRole) {
    let mut state = self.state.lock().expect(LOCK_ERROR);
    state.pending_roles.entry(room.to_string()).or_default().push_back(role);
  }

  /// Consumes the next approved role for a room.
  fn take_pending_role(&mut self, room: &str) -> Option<PeerRole> {
    let mut state = self.state.lock().expect(LOCK_ERROR);
    let roles = state.pending_roles.get_mut(room)?;
    let role = roles.pop_front();
    if roles.is_empty() {
      state.pending_roles.remove(room);
    }
    role
  }

  /// Registers a peer as room host when no host exists.
  fn add_host(&mut self, room: &str, peer_id: PeerId, sender: SignalingChannel) -> bool {
    let mut state = self.state.lock().expect(LOCK_ERROR);
    let room_state = state.rooms.entry(room.to_string()).or_default();
    if room_state.host.is_some() {
      return false;
    }
    room_state.host = Some((peer_id, sender));
    state.peers.insert(
      peer_id,
      PeerRoomMembership {
        room: room.to_string(),
        role: PeerRole::Host,
      },
    );
    true
  }

  /// Registers a client and notifies that room's host.
  fn add_client(
    &mut self,
    room: &str,
    peer_id: PeerId,
    sender: SignalingChannel,
  ) -> Result<(), matchbox_signaling::SignalingError> {
    let host_sender = {
      let state = self.state.lock().expect(LOCK_ERROR);
      state
        .rooms
        .get(room)
        .and_then(|room_state| room_state.host.as_ref())
        .map(|(_host_id, host_sender)| host_sender.clone())
        .ok_or(matchbox_signaling::SignalingError::UnknownPeer)?
    };

    try_send(
      &host_sender,
      Message::Text(JsonPeerEvent::NewPeer(peer_id).to_string().into()),
    )?;

    let mut state = self.state.lock().expect(LOCK_ERROR);
    let room_state = state
      .rooms
      .get_mut(room)
      .ok_or(matchbox_signaling::SignalingError::UnknownPeer)?;
    room_state.clients.insert(peer_id, sender);
    state.peers.insert(
      peer_id,
      PeerRoomMembership {
        room: room.to_string(),
        role: PeerRole::Client,
      },
    );
    Ok(())
  }

  /// Sends a signalling event to a room's host.
  fn try_send_to_host(&self, room: &str, message: Message) -> Result<(), matchbox_signaling::SignalingError> {
    let host_sender = {
      let state = self.state.lock().expect(LOCK_ERROR);
      state
        .rooms
        .get(room)
        .and_then(|room_state| room_state.host.as_ref())
        .map(|(_host_id, sender)| sender.clone())
        .ok_or(matchbox_signaling::SignalingError::UnknownPeer)?
    };
    try_send(&host_sender, message)
  }

  /// Sends a signalling event to one client in a room.
  fn try_send_to_client(
    &self,
    room: &str,
    peer_id: PeerId,
    message: Message,
  ) -> Result<(), matchbox_signaling::SignalingError> {
    let client_sender = {
      let state = self.state.lock().expect(LOCK_ERROR);
      state
        .rooms
        .get(room)
        .and_then(|room_state| room_state.clients.get(&peer_id))
        .cloned()
        .ok_or(matchbox_signaling::SignalingError::UnknownPeer)?
    };
    try_send(&client_sender, message)
  }

  /// Removes a peer according to its role and emits lifecycle callbacks.
  fn disconnect_peer(
    &mut self,
    room: &str,
    peer_id: PeerId,
    role: PeerRole,
    callbacks: &RoomAwareClientServerCallbacks,
  ) {
    match role {
      PeerRole::Host => {
        self.disconnect_host(room, peer_id);
        info!("Host [{peer_id}] left");
        callbacks.host_disconnected.emit(peer_id);
      }
      PeerRole::Client => {
        self.disconnect_client(room, peer_id);
        info!("Client [{peer_id}] left");
        callbacks.client_disconnected.emit(peer_id);
      }
    }
  }

  /// Removes one client and notifies only its room host.
  fn disconnect_client(&mut self, room: &str, peer_id: PeerId) {
    let host_sender = {
      let mut state = self.state.lock().expect(LOCK_ERROR);
      let Some(membership) = state.peers.remove(&peer_id) else {
        return;
      };
      if membership.room != room || !matches!(membership.role, PeerRole::Client) {
        return;
      }
      state.rooms.get_mut(room).and_then(|room_state| {
        room_state.clients.remove(&peer_id);
        room_state.host.as_ref().map(|(_host_id, sender)| sender.clone())
      })
    };

    if let Some(host_sender) = host_sender {
      let event = Message::Text(JsonPeerEvent::PeerLeft(peer_id).to_string().into());
      if let Err(error) = try_send(&host_sender, event) {
        error!("Failure sending peer remove to host: {error:?}");
      }
    }
  }

  /// Removes one host, clears its room, and notifies its clients.
  fn disconnect_host(&mut self, room: &str, peer_id: PeerId) {
    let client_senders = {
      let mut state = self.state.lock().expect(LOCK_ERROR);
      let Some(room_state) = state.rooms.get(room) else {
        return;
      };
      if !matches!(room_state.host, Some((host_id, _)) if host_id == peer_id) {
        return;
      }
      let Some(mut room_state) = state.rooms.remove(room) else {
        return;
      };
      state.peers.remove(&peer_id);
      for client_id in room_state.clients.keys() {
        state.peers.remove(client_id);
      }
      room_state
        .clients
        .drain()
        .map(|(_client_id, sender)| sender)
        .collect::<Vec<_>>()
    };

    let event = Message::Text(JsonPeerEvent::PeerLeft(peer_id).to_string().into());
    for sender in client_senders {
      if let Err(error) = try_send(&sender, event.clone()) {
        error!("Failure sending host remove to client: {error:?}");
      }
    }
  }
}

#[expect(
  clippy::result_large_err,
  reason = "matchbox_signaling requires axum::response::Response for connection rejection"
)]
/// Parses the optional `role` query parameter.
fn parse_role(query_params: &HashMap<String, String>) -> Result<PeerRole, Response> {
  match query_params.get("role").map(String::as_str) {
    None | Some("" | "client") => Ok(PeerRole::Client),
    Some("host") => Ok(PeerRole::Host),
    Some(_) => Err((StatusCode::BAD_REQUEST, "Role must be 'host' or 'client'\n").into_response()),
  }
}

/// Logs a failed client WebSocket request.
fn log_client_request_error(peer_id: PeerId, error: &ClientRequestError) {
  match error {
    ClientRequestError::Axum(_) => warn!("Unrecoverable error with {peer_id}: {error:?}"),
    ClientRequestError::Close => info!("Connection closed by {peer_id}"),
    ClientRequestError::Json(_) | ClientRequestError::UnsupportedType(_) => error!("Error with request: {error:?}"),
  }
}
