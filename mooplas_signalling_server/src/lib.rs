#![deny(clippy::all, clippy::pedantic)]

use axum::{
  Router,
  extract::connect_info::IntoMakeServiceWithConnectInfo,
  routing::get,
  serve::{Listener, ListenerExt},
};
use matchbox_signaling::SignalingServer;
use std::{
  error::Error as StdError,
  fmt,
  fs::File,
  io::{self, BufReader},
  net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener as StdTcpListener},
  path::{Path, PathBuf},
  sync::Arc,
  time::Duration,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{
  TlsAcceptor,
  rustls::{
    ServerConfig as RustlsServerConfig,
    crypto::{CryptoProvider, aws_lc_rs},
    pki_types::{CertificateDer, PrivateKeyDer},
  },
  server::TlsStream,
};
use tracing::*;

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

pub struct StandaloneServer {
  requested_addr: SocketAddr,
  listener: Option<StdTcpListener>,
  service: IntoMakeServiceWithConnectInfo<Router, SocketAddr>,
  tls_acceptor: Option<TlsAcceptor>,
}

impl fmt::Debug for StandaloneServer {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("StandaloneServer")
      .field("requested_addr", &self.requested_addr)
      .field("local_addr", &self.local_addr())
      .field("tls_enabled", &self.tls_acceptor.is_some())
      .finish_non_exhaustive()
  }
}

impl StandaloneServer {
  #[must_use]
  pub fn local_addr(&self) -> Option<SocketAddr> {
    self.listener.as_ref().and_then(|listener| listener.local_addr().ok())
  }

  /// Binds the configured server so tests or callers can inspect the chosen socket address.
  ///
  /// # Errors
  ///
  /// Returns an error if the TCP listener cannot be created or configured.
  pub fn bind(&mut self) -> Result<SocketAddr, ServerError> {
    let listener = StdTcpListener::bind(self.requested_addr).map_err(ServerError::Bind)?;
    listener.set_nonblocking(true).map_err(ServerError::Bind)?;
    let addr = listener.local_addr().map_err(ServerError::Bind)?;
    self.listener = Some(listener);
    Ok(addr)
  }

  /// Runs the configured signalling server until it is stopped.
  ///
  /// # Errors
  ///
  /// Returns an error if the TCP listener cannot be created or if axum stops serving unexpectedly.
  pub async fn serve(mut self) -> Result<(), ServerError> {
    if self.listener.is_none() {
      let _ = self.bind()?;
    }
    let Some(listener) = self.listener.take() else {
      unreachable!("listener should be present after binding");
    };
    let listener = TcpListener::from_std(listener).map_err(ServerError::Bind)?;
    match self.tls_acceptor {
      Some(tls_acceptor) => axum::serve(TlsListener::new(listener, tls_acceptor).tap_io(|_| {}), self.service)
        .await
        .map_err(ServerError::Serve)?,
      None => axum::serve(listener, self.service).await.map_err(ServerError::Serve)?,
    }
    Ok(())
  }
}

#[derive(Debug)]
pub enum ServerError {
  Bind(io::Error),
  Serve(io::Error),
  LoadTlsCertificates { path: PathBuf, source: io::Error },
  MissingTlsCertificates { path: PathBuf },
  LoadTlsPrivateKey { path: PathBuf, source: io::Error },
  MissingTlsPrivateKey { path: PathBuf },
  ConfigureTls(tokio_rustls::rustls::Error),
}

impl fmt::Display for ServerError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Bind(error) => write!(formatter, "Failed to bind signalling server: {error}"),
      Self::Serve(error) => write!(formatter, "Signalling server stopped unexpectedly: {error}"),
      Self::LoadTlsCertificates { path, source } => {
        write!(
          formatter,
          "Failed to load TLS certificates from [{}]: {source}",
          path.display()
        )
      }
      Self::MissingTlsCertificates { path } => {
        write!(formatter, "No TLS certificates found in [{}]", path.display())
      }
      Self::LoadTlsPrivateKey { path, source } => {
        write!(
          formatter,
          "Failed to load TLS private key from [{}]: {source}",
          path.display()
        )
      }
      Self::MissingTlsPrivateKey { path } => {
        write!(formatter, "No TLS private key found in [{}]", path.display())
      }
      Self::ConfigureTls(error) => write!(formatter, "Failed to configure TLS: {error}"),
    }
  }
}

impl StdError for ServerError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      Self::Bind(source)
      | Self::Serve(source)
      | Self::LoadTlsCertificates { source, .. }
      | Self::LoadTlsPrivateKey { source, .. } => Some(source),
      Self::ConfigureTls(source) => Some(source),
      Self::MissingTlsCertificates { .. } | Self::MissingTlsPrivateKey { .. } => None,
    }
  }
}

struct TlsListener {
  listener: TcpListener,
  acceptor: TlsAcceptor,
}

impl TlsListener {
  const ACCEPT_RETRY_DELAY: Duration = Duration::from_secs(1);

  const fn new(listener: TcpListener, acceptor: TlsAcceptor) -> Self {
    Self { listener, acceptor }
  }
}

impl Listener for TlsListener {
  type Io = TlsStream<TcpStream>;
  type Addr = SocketAddr;

  async fn accept(&mut self) -> (Self::Io, Self::Addr) {
    loop {
      match self.listener.accept().await {
        Ok((stream, remote_addr)) => match self.acceptor.accept(stream).await {
          Ok(tls_stream) => return (tls_stream, remote_addr),
          Err(error) => warn!("TLS handshake rejected for [{remote_addr}]: {error}"),
        },
        Err(error) => {
          error!("Accept error: {error}");
          tokio::time::sleep(Self::ACCEPT_RETRY_DELAY).await;
        }
      }
    }
  }

  fn local_addr(&self) -> io::Result<Self::Addr> {
    self.listener.local_addr()
  }
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
  Ok(StandaloneServer {
    requested_addr,
    listener: None,
    service,
    tls_acceptor,
  })
}

/// Runs the standalone signalling server until it is stopped.
///
/// # Errors
///
/// Returns an error if the server cannot bind, if TLS material is invalid, or if axum fails while serving requests.
pub async fn run_server(config: ServerConfig) -> Result<(), ServerError> {
  build_server(config)?.serve().await
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
      info!("Connecting: {connection:?}");
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
  "ok"
}
