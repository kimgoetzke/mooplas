use crate::error::ServerError;
use axum::Router;
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::serve::{Listener, ListenerExt};
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::time::Duration;
use std::{fmt, io};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use tracing::{error, warn};

pub struct StandaloneServer {
  pub requested_addr: SocketAddr,
  pub listener: Option<StdTcpListener>,
  pub service: IntoMakeServiceWithConnectInfo<Router, SocketAddr>,
  pub tls_acceptor: Option<TlsAcceptor>,
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
