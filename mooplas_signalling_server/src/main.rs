use clap::Parser;
use mooplas_signalling_server::{DEFAULT_PORT, ServerConfig, TlsConfig, run_server};
use std::path::PathBuf;
use tracing_subscriber::prelude::*;

#[derive(Debug, Parser)]
#[command(about = "Standalone signalling server for mooplas")]
struct Cli {
  #[arg(long, default_value_t = DEFAULT_PORT)]
  port: u16,
  #[arg(long, requires = "tls_key")]
  tls_cert: Option<PathBuf>,
  #[arg(long, requires = "tls_cert")]
  tls_key: Option<PathBuf>,
}

impl Cli {
  fn server_config(self) -> ServerConfig {
    ServerConfig {
      port: self.port,
      tls: self
        .tls_cert
        .zip(self.tls_key)
        .map(|(cert_path, key_path)| TlsConfig { cert_path, key_path }),
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), mooplas_signalling_server::ServerError> {
  setup_logging();
  run_server(Cli::parse().server_config()).await
}

fn setup_logging() {
  tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
    .with(tracing_subscriber::fmt::layer())
    .init();
}

#[cfg(test)]
mod tests {
  use clap::Parser;
  use std::path::PathBuf;

  use super::Cli;

  #[test]
  fn cli_uses_default_port_when_not_provided() {
    let cli = Cli::try_parse_from(["mooplas_signalling_server"]).expect("parse cli arguments");
    assert_eq!(cli.port, mooplas_signalling_server::DEFAULT_PORT);
  }

  #[test]
  fn cli_accepts_custom_port() {
    let cli = Cli::try_parse_from(["mooplas_signalling_server", "--port", "4545"]).expect("parse cli arguments");
    assert_eq!(cli.port, 4545);
  }

  #[test]
  fn cli_accepts_tls_paths_when_both_are_provided() {
    let cli = Cli::try_parse_from([
      "mooplas_signalling_server",
      "--tls-cert",
      "cert.pem",
      "--tls-key",
      "key.pem",
    ])
    .expect("parse cli arguments");
    assert_eq!(
      cli.server_config(),
      mooplas_signalling_server::ServerConfig {
        port: mooplas_signalling_server::DEFAULT_PORT,
        tls: Some(mooplas_signalling_server::TlsConfig {
          cert_path: PathBuf::from("cert.pem"),
          key_path: PathBuf::from("key.pem"),
        }),
      }
    );
  }

  #[test]
  fn cli_rejects_tls_certificate_without_private_key() {
    let error = Cli::try_parse_from(["mooplas_signalling_server", "--tls-cert", "cert.pem"])
      .expect_err("parsing should fail without --tls-key");
    assert_eq!(error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
  }

  #[test]
  fn cli_rejects_tls_private_key_without_certificate() {
    let error = Cli::try_parse_from(["mooplas_signalling_server", "--tls-key", "key.pem"])
      .expect_err("parsing should fail without --tls-cert");
    assert_eq!(error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
  }
}
