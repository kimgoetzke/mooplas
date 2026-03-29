use std::error::Error as StdError;
use std::path::PathBuf;
use std::{fmt, io};

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
