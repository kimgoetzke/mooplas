use std::{path::PathBuf, time::Duration};

use mooplas_signalling_server::{ServerConfig, TlsConfig, build_server};

const TLS_CERT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tls-cert.pem");
const TLS_KEY_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tls-key.pem");

async fn wait_for_health(client: &reqwest::Client, url: &str) -> reqwest::Response {
  for _ in 0..20 {
    if let Ok(response) = client.get(url).send().await {
      return response;
    }

    tokio::time::sleep(Duration::from_millis(25)).await;
  }

  panic!("health endpoint never responded");
}

#[tokio::test]
async fn health_endpoint_responds_when_server_is_running() {
  let mut server = build_server(ServerConfig { port: 0, tls: None }).expect("build signalling server");
  let socket_addr = server.bind().expect("bind signalling server");
  let server_handle = tokio::spawn(server.serve());
  let client = reqwest::Client::new();

  let response = wait_for_health(&client, &format!("http://{socket_addr}/health")).await;

  assert_eq!(response.status(), reqwest::StatusCode::OK);
  assert_eq!(response.text().await.expect("read health response"), "ok");

  server_handle.abort();
  let _ = server_handle.await;
}

#[tokio::test]
async fn https_health_endpoint_responds_when_tls_is_enabled() {
  let mut server = build_server(ServerConfig {
    port: 0,
    tls: Some(TlsConfig {
      cert_path: PathBuf::from(TLS_CERT_PATH),
      key_path: PathBuf::from(TLS_KEY_PATH),
    }),
  })
  .expect("build signalling server");
  let socket_addr = server.bind().expect("bind signalling server");
  let server_handle = tokio::spawn(server.serve());
  let client = reqwest::Client::builder()
    .resolve("foobar.com", socket_addr)
    .tls_danger_accept_invalid_certs(true)
    .build()
    .expect("build reqwest client");

  let response = wait_for_health(&client, &format!("https://foobar.com:{}/health", socket_addr.port())).await;

  assert_eq!(response.status(), reqwest::StatusCode::OK);
  assert_eq!(response.text().await.expect("read health response"), "ok");

  server_handle.abort();
  let _ = server_handle.await;
}
