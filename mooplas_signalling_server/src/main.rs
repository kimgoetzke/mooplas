use clap::Parser;
use mooplas_signalling_server::{DEFAULT_PORT, ServerConfig, run_server};
use tracing_subscriber::prelude::*;

#[derive(Debug, Parser)]
#[command(about = "Standalone signalling server for mooplas")]
struct Cli {
  #[arg(long, default_value_t = DEFAULT_PORT)]
  port: u16,
}

#[tokio::main]
async fn main() -> Result<(), matchbox_signaling::Error> {
  setup_logging();
  let cli = Cli::parse();
  run_server(ServerConfig { port: cli.port }).await
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
}
