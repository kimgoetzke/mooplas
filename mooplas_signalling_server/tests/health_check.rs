use std::time::Duration;

use mooplas_signalling_server::{ServerConfig, build_server};

async fn wait_for_health(url: &str) -> reqwest::Response {
  let client = reqwest::Client::new();

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
  let mut server = build_server(ServerConfig { port: 0 });
  let socket_addr = server.bind().expect("bind signalling server");
  let server_handle = tokio::spawn(server.serve());

  let response = wait_for_health(&format!("http://{socket_addr}/health")).await;

  assert_eq!(response.status(), reqwest::StatusCode::OK);
  assert_eq!(response.text().await.expect("read health response"), "ok");

  server_handle.abort();
  let _ = server_handle.await;
}
