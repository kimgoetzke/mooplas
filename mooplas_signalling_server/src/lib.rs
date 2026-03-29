#![deny(clippy::all, clippy::pedantic)]

use axum::{Router, routing::get};
use error::ServerError;
use matchbox_signaling::SignalingServer;
use server::StandaloneServer;
use std::{
  fs::File,
  io::BufReader,
  net::{Ipv4Addr, SocketAddr, SocketAddrV4},
  path::{Path, PathBuf},
  sync::Arc,
};
use tokio_rustls::{
  TlsAcceptor,
  rustls::{
    ServerConfig as RustlsServerConfig,
    crypto::{CryptoProvider, aws_lc_rs},
    pki_types::{CertificateDer, PrivateKeyDer},
  },
};
use tracing::*;

pub mod error;
mod server;

pub const DEFAULT_PORT: u16 = 3536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsConfig {
  pub cert_path: PathBuf,
  pub key_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
  pub port: u16,
  pub tls: Option<TlsConfig>,
}

impl Default for ServerConfig {
  fn default() -> Self {
    Self {
      port: DEFAULT_PORT,
      tls: None,
    }
  }
}

/// Runs the standalone signalling server until it is stopped.
///
/// # Errors
///
/// Returns an error if the server cannot bind, if TLS material is invalid, or if axum fails while serving requests.
pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
  build_server(config)?.serve().await
}

/// Builds a standalone Matchbox signalling server using client/server topology, with optional TLS termination.
///
/// # Errors
///
/// Returns an error if the TLS certificate or key cannot be loaded when TLS is configured.
pub fn build_server(config: ServerConfig) -> Result<StandaloneServer, ServerError> {
  let ServerConfig { port, tls } = config;
  let requested_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port));
  let service = build_router(requested_addr).into_make_service_with_connect_info::<SocketAddr>();
  let tls_acceptor = tls.as_ref().map(load_tls_acceptor).transpose()?;
  info!("Server listening on port [{}]...", port);
  Ok(StandaloneServer {
    requested_addr,
    listener: None,
    service,
    tls_acceptor,
  })
}

#[expect(
  clippy::result_large_err,
  reason = "matchbox_signaling requires axum::response::Response for connection rejection"
)]
fn build_router(requested_addr: SocketAddr) -> Router {
  let mut captured_router = None;
  let _ = SignalingServer::client_server_builder(requested_addr)
    .mutate_router(|router| router.route("/health", get(health_check)))
    .on_connection_request(|connection| {
      info!("Connecting: {connection:?}...");
      Ok(true)
    })
    .on_id_assignment(|(socket, id)| info!("Socket [{socket}] received ID [{id}]"))
    .on_host_connected(|id| info!("Host joined and has ID [{id}]"))
    .on_host_disconnected(|id| info!("Host [{id}] left"))
    .on_client_connected(|id| info!("Client with ID [{id}] connected"))
    .on_client_disconnected(|id| info!("Client [{id}] left"))
    .cors()
    .trace()
    .build_with(|router| {
      captured_router = Some(router.clone());
      router
    });
  captured_router.expect("The matchbox_signaling builder failed to expose the configured router")
}

fn load_tls_acceptor(config: &TlsConfig) -> Result<TlsAcceptor, ServerError> {
  ensure_tls_crypto_provider_exists();
  let certificates = load_tls_certificates(&config.cert_path)?;
  let private_key = load_tls_private_key(&config.key_path)?;
  let server_config = RustlsServerConfig::builder()
    .with_no_client_auth()
    .with_single_cert(certificates, private_key)
    .map_err(ServerError::ConfigureTls)?;
  Ok(TlsAcceptor::from(Arc::new(server_config)))
}

fn ensure_tls_crypto_provider_exists() {
  if CryptoProvider::get_default().is_none() {
    let _ = aws_lc_rs::default_provider().install_default();
  }
}

fn load_tls_certificates(path: &Path) -> Result<Vec<CertificateDer<'static>>, ServerError> {
  let file = File::open(path).map_err(|source| ServerError::LoadTlsCertificates {
    path: path.to_path_buf(),
    source,
  })?;
  let mut reader = BufReader::new(file);
  let certificates = rustls_pemfile::certs(&mut reader)
    .collect::<Result<Vec<_>, _>>()
    .map_err(|source| ServerError::LoadTlsCertificates {
      path: path.to_path_buf(),
      source,
    })?;
  if certificates.is_empty() {
    return Err(ServerError::MissingTlsCertificates {
      path: path.to_path_buf(),
    });
  }
  Ok(certificates)
}

fn load_tls_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, ServerError> {
  let file = File::open(path).map_err(|source| ServerError::LoadTlsPrivateKey {
    path: path.to_path_buf(),
    source,
  })?;
  let mut reader = BufReader::new(file);
  let private_key = rustls_pemfile::private_key(&mut reader).map_err(|source| ServerError::LoadTlsPrivateKey {
    path: path.to_path_buf(),
    source,
  })?;
  private_key.ok_or_else(|| ServerError::MissingTlsPrivateKey {
    path: path.to_path_buf(),
  })
}

async fn health_check() -> &'static str {
  "OK\n"
}
