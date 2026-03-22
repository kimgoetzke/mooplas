#![deny(clippy::all, clippy::pedantic)]

use axum::routing::get;
use matchbox_signaling::{Error, SignalingServer};
use std::net::{Ipv4Addr, SocketAddrV4};
use tracing::{info, trace};

pub const DEFAULT_PORT: u16 = 3536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerConfig {
  pub port: u16,
}

impl Default for ServerConfig {
  fn default() -> Self {
    Self { port: DEFAULT_PORT }
  }
}

/// Builds a standalone Matchbox signalling server using client/server topology.
#[must_use]
#[expect(
  clippy::result_large_err,
  reason = "matchbox_signaling requires axum::response::Response for connection rejection"
)]
pub fn build_server(config: ServerConfig) -> SignalingServer {
  SignalingServer::client_server_builder(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, config.port))
    .mutate_router(|router| router.route("/health", get(health_check)))
    .on_connection_request(|connection| {
      info!("Connecting: {connection:?}");
      Ok(true)
    })
    .on_id_assignment(|(socket, id)| info!("{socket} received {id}"))
    .on_host_connected(|id| info!("Host joined: {id}"))
    .on_host_disconnected(|id| info!("Host left: {id}"))
    .on_client_connected(|id| trace!("Client joined: {id}"))
    .on_client_disconnected(|id| trace!("Client left: {id}"))
    .cors()
    .trace()
    .build()
}

/// Runs the standalone signalling server until it is stopped.
///
/// # Errors
///
/// Returns an error if the server cannot bind or if axum fails while serving requests.
pub async fn run_server(config: ServerConfig) -> Result<(), Error> {
  build_server(config).serve().await
}

async fn health_check() -> &'static str {
  "ok"
}
