use std::time::Duration;

use dprint_markdown_ja_formatter_core::Formatter;
use dprint_markdown_ja_formatter_mcp::{Options, router};
use reqwest::{Client, Response, StatusCode, header};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;

struct TestServer {
  base_url: String,
  cancellation: CancellationToken,
  task: JoinHandle<()>,
}

impl Drop for TestServer {
  fn drop(&mut self) {
    self.cancellation.cancel();
    self.task.abort();
  }
}

async fn spawn_server(max_request_bytes: usize) -> TestServer {
  let cancellation = CancellationToken::new();
  let options = Options {
    listen: "127.0.0.1:0".parse().unwrap(),
    allowed_host: vec!["mcp.test".to_owned()],
    max_request_bytes,
    timeout_seconds: 10,
    max_concurrency: 2,
  };
  let app = router(&options, cancellation.clone()).unwrap();
  let listener = TcpListener::bind(options.listen).await.unwrap();
  let address = listener.local_addr().unwrap();
  let shutdown = cancellation.clone();
  let task = tokio::spawn(async move {
    axum::serve(listener, app)
      .with_graceful_shutdown(shutdown.cancelled_owned())
      .await
      .unwrap();
  });
  TestServer {
    base_url: format!("http://{address}"),
    cancellation,
    task,
  }
}

async fn post(client: &Client, server: &TestServer, body: Value, session: Option<&str>) -> Response {
  let mut request = client
    .post(format!("{}/mcp", server.base_url))
    .header(header::HOST, "mcp.test")
    .header(header::ACCEPT, "application/json, text/event-stream")
    .header(header::CONTENT_TYPE, "application/json")
    .header("MCP-Protocol-Version", "2025-11-25")
    .body(body.to_string());
  if let Some(session) = session {
    request = request.header("Mcp-Session-Id", session);
  }
  request.send().await.unwrap()
}

async fn response_json(response: Response) -> Value {
  let body = response.text().await.unwrap();
  if let Ok(value) = serde_json::from_str(&body) {
    return value;
  }
  let data = body
    .lines()
    .filter_map(|line| line.strip_prefix("data:").map(str::trim))
    .find(|data| !data.is_empty())
    .unwrap_or_else(|| panic!("response contained neither JSON nor SSE data: {body:?}"));
  serde_json::from_str(data).unwrap()
}

#[tokio::test]
async fn protocol_sequence_formats_with_defaults_and_options() {
  let server = spawn_server(1024 * 1024).await;
  let client = Client::new();

  let response = post(
    &client,
    &server,
    json!({
      "jsonrpc": "2.0",
      "id": 1,
      "method": "initialize",
      "params": {
        "protocolVersion": "2025-11-25",
        "capabilities": {},
        "clientInfo": {"name": "integration-test", "version": "1.0"}
      }
    }),
    None,
  )
  .await;
  assert_eq!(response.status(), StatusCode::OK);
  let session = response
    .headers()
    .get("Mcp-Session-Id")
    .expect("initialize session id")
    .to_str()
    .unwrap()
    .to_owned();
  let body = response_json(response).await;
  assert_eq!(body["id"], 1);
  assert_eq!(body["result"]["protocolVersion"], "2025-11-25");
  assert!(body["result"]["capabilities"]["tools"].is_object());

  let response = post(
    &client,
    &server,
    json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    Some(&session),
  )
  .await;
  assert_eq!(response.status(), StatusCode::ACCEPTED);

  let response = post(
    &client,
    &server,
    json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
    Some(&session),
  )
  .await;
  assert_eq!(response.status(), StatusCode::OK);
  let body = response_json(response).await;
  let tools = body["result"]["tools"].as_array().unwrap();
  assert_eq!(tools.len(), 1);
  assert_eq!(tools[0]["name"], "format_markdown");
  assert_eq!(tools[0]["inputSchema"]["required"], json!(["markdown"]));
  assert!(tools[0]["outputSchema"].is_object());
  assert_eq!(tools[0]["annotations"]["readOnlyHint"], true);

  let input = "日本語English *text*";
  let response = post(
    &client,
    &server,
    json!({
      "jsonrpc": "2.0",
      "id": 3,
      "method": "tools/call",
      "params": {"name": "format_markdown", "arguments": {"markdown": input}}
    }),
    Some(&session),
  )
  .await;
  assert_eq!(response.status(), StatusCode::OK);
  let body = response_json(response).await;
  let result = &body["result"]["structuredContent"];
  assert_eq!(result["markdown"], Formatter::default().format(input).unwrap().as_ref());
  assert_eq!(result["changed"], true);
  assert_eq!(body["result"]["isError"], false);

  let option_input = "alpha beta gamma delta epsilon\n\n_emphasis_ **strong**\n";
  let expected = Formatter::new(16, "always", "asterisks", "underscores")
    .unwrap()
    .format(option_input)
    .unwrap()
    .into_owned();
  let response = post(
    &client,
    &server,
    json!({
      "jsonrpc": "2.0",
      "id": 4,
      "method": "tools/call",
      "params": {
        "name": "format_markdown",
        "arguments": {
          "markdown": option_input,
          "lineWidth": 16,
          "textWrap": "always",
          "emphasis": "asterisks",
          "strong": "underscores"
        }
      }
    }),
    Some(&session),
  )
  .await;
  assert_eq!(response.status(), StatusCode::OK);
  let body = response_json(response).await;
  assert_eq!(body["result"]["structuredContent"]["markdown"], expected);
}

#[tokio::test]
async fn health_and_http_boundary_checks() {
  let server = spawn_server(512).await;
  let client = Client::builder().timeout(Duration::from_secs(5)).build().unwrap();

  let response = client.get(format!("{}/healthz", server.base_url)).send().await.unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(response.text().await.unwrap(), "ok\n");

  let request = json!({
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-11-25",
      "capabilities": {},
      "clientInfo": {"name": "test", "version": "1"}
    }
  });
  let response = client
    .post(format!("{}/mcp", server.base_url))
    .header(header::HOST, "untrusted.example")
    .header(header::ACCEPT, "application/json, text/event-stream")
    .header(header::CONTENT_TYPE, "application/json")
    .json(&request)
    .send()
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::FORBIDDEN);

  let response = client
    .post(format!("{}/mcp", server.base_url))
    .header(header::HOST, "mcp.test")
    .header(header::ORIGIN, "https://untrusted.example")
    .header(header::ACCEPT, "application/json, text/event-stream")
    .header(header::CONTENT_TYPE, "application/json")
    .json(&request)
    .send()
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::FORBIDDEN);

  let response = client
    .post(format!("{}/mcp", server.base_url))
    .header(header::HOST, "mcp.test")
    .header(header::ACCEPT, "application/json, text/event-stream")
    .header(header::CONTENT_TYPE, "application/json")
    .body("x".repeat(513))
    .send()
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
