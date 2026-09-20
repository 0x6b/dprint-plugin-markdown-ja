use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use axum::{Router, http::StatusCode, routing::get};
use clap::Parser;
use dprint_markdown_ja_formatter_core::Formatter;
use rmcp::{
  ErrorData as McpError, Json, ServerHandler,
  handler::server::{router::tool::ToolRouter, wrapper::Parameters},
  model::{Implementation, ServerCapabilities, ServerConfig},
  schemars::{self, JsonSchema},
  tool, tool_handler, tool_router,
  transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
  },
};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::Semaphore, task::spawn_blocking, time::timeout};
use tokio_util::sync::CancellationToken;

const DEFAULT_MAX_REQUEST_BYTES: usize = 1024 * 1024;
const DEFAULT_TIMEOUT_SECONDS: u64 = 10;
const DEFAULT_MAX_CONCURRENCY: usize = 4;

#[derive(Debug, Parser)]
#[command(
  name = "dprint-markdown-ja-formatter-mcp",
  version,
  about = "Serve Japanese-aware Markdown formatting over Streamable HTTP MCP"
)]
pub struct Options {
  /// HTTP listen address
  #[arg(long, default_value = "127.0.0.1:3000")]
  pub listen: SocketAddr,

  /// Allow an exact Host authority (repeatable); required for non-loopback listeners
  #[arg(long, value_name = "HOST[:PORT]")]
  pub allowed_host: Vec<String>,

  /// Maximum JSON request body in bytes
  #[arg(long, default_value_t = DEFAULT_MAX_REQUEST_BYTES)]
  pub max_request_bytes: usize,

  /// Maximum duration of one formatting call
  #[arg(long, default_value_t = DEFAULT_TIMEOUT_SECONDS)]
  pub timeout_seconds: u64,

  /// Maximum number of formatting calls executing at once
  #[arg(long, default_value_t = DEFAULT_MAX_CONCURRENCY)]
  pub max_concurrency: usize,
}

impl Options {
  fn validate(&self) -> Result<()> {
    if !self.listen.ip().is_loopback() && self.allowed_host.is_empty() {
      bail!("--allowed-host is required for a non-loopback listener");
    }
    ensure!(
      self.max_request_bytes > 0,
      "--max-request-bytes must be greater than zero"
    );
    ensure!(self.timeout_seconds > 0, "--timeout-seconds must be greater than zero");
    ensure!(self.max_concurrency > 0, "--max-concurrency must be greater than zero");
    Ok(())
  }
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TextWrapOption {
  Never,
  Maintain,
  Always,
}

impl TextWrapOption {
  fn as_str(self) -> &'static str {
    match self {
      Self::Never => "never",
      Self::Maintain => "maintain",
      Self::Always => "always",
    }
  }
}

#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MarkerOption {
  Underscores,
  Asterisks,
}

impl MarkerOption {
  fn as_str(self) -> &'static str {
    match self {
      Self::Underscores => "underscores",
      Self::Asterisks => "asterisks",
    }
  }
}

const fn default_line_width() -> i32 {
  80
}

const fn default_text_wrap() -> TextWrapOption {
  TextWrapOption::Never
}

const fn default_emphasis() -> MarkerOption {
  MarkerOption::Underscores
}

const fn default_strong() -> MarkerOption {
  MarkerOption::Asterisks
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FormatMarkdownInput {
  /// Markdown source text to format.
  pub markdown: String,

  /// Markdown line width (1..10000).
  #[serde(default = "default_line_width")]
  #[schemars(default = "default_line_width")]
  pub line_width: i32,

  /// Text wrapping mode.
  #[serde(default = "default_text_wrap")]
  #[schemars(default = "default_text_wrap")]
  pub text_wrap: TextWrapOption,

  /// Emphasis marker style.
  #[serde(default = "default_emphasis")]
  #[schemars(default = "default_emphasis")]
  pub emphasis: MarkerOption,

  /// Strong marker style.
  #[serde(default = "default_strong")]
  #[schemars(default = "default_strong")]
  pub strong: MarkerOption,
}

#[derive(Debug, JsonSchema, Serialize)]
pub struct FormatMarkdownOutput {
  pub markdown: String,
  pub changed: bool,
}

#[derive(Clone)]
struct MarkdownServer {
  tool_router: ToolRouter<Self>,
  permits: Arc<Semaphore>,
  deadline: Duration,
}

impl MarkdownServer {
  fn new(max_concurrency: usize, deadline: Duration) -> Self {
    Self {
      tool_router: Self::tool_router(),
      permits: Arc::new(Semaphore::new(max_concurrency)),
      deadline,
    }
  }
}

#[tool_router(router = tool_router)]
impl MarkdownServer {
  #[tool(
    description = "Format Japanese-aware Markdown and return the complete formatted text without reading or writing files",
    annotations(
      read_only_hint = true,
      destructive_hint = false,
      idempotent_hint = true,
      open_world_hint = false
    )
  )]
  async fn format_markdown(
    &self,
    Parameters(input): Parameters<FormatMarkdownInput>,
  ) -> Result<Json<FormatMarkdownOutput>, McpError> {
    let permit = self
      .permits
      .clone()
      .try_acquire_owned()
      .map_err(|_| McpError::internal_error("formatter concurrency limit reached", None))?;
    let task = spawn_blocking(move || {
      let _permit = permit;
      format_markdown(input)
    });

    match timeout(self.deadline, task).await {
      Ok(Ok(Ok(output))) => Ok(Json(output)),
      Ok(Ok(Err(error))) => Err(McpError::invalid_params(error.to_string(), None)),
      Ok(Err(_)) => Err(McpError::internal_error("formatter worker failed", None)),
      Err(_) => Err(McpError::internal_error("formatting timed out", None)),
    }
  }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for MarkdownServer {
  fn get_info(&self) -> ServerConfig {
    ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
      .with_server_info(Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
      .with_instructions("Format supplied Markdown text. This server never reads or writes files.")
  }
}

fn format_markdown(input: FormatMarkdownInput) -> Result<FormatMarkdownOutput> {
  let formatter = Formatter::new(
    input.line_width,
    input.text_wrap.as_str(),
    input.emphasis.as_str(),
    input.strong.as_str(),
  )?;
  let formatted = formatter.format(&input.markdown)?;
  let changed = formatted != input.markdown;
  Ok(FormatMarkdownOutput {
    markdown: formatted.into_owned(),
    changed,
  })
}

pub fn router(options: &Options, cancellation: CancellationToken) -> Result<Router> {
  options.validate()?;
  let server = MarkdownServer::new(options.max_concurrency, Duration::from_secs(options.timeout_seconds));
  let mut config = StreamableHttpServerConfig::default()
    .with_json_response(true)
    .with_max_request_body_bytes(options.max_request_bytes)
    .enforce_origin_validation()
    .with_cancellation_token(cancellation.child_token());
  if !options.allowed_host.is_empty() {
    config = config.with_allowed_hosts(options.allowed_host.clone());
  }
  let service = StreamableHttpService::new(
    move || Ok(server.clone()),
    Arc::new(LocalSessionManager::default()),
    config,
  );

  Ok(
    Router::new()
      .route("/healthz", get(|| async { (StatusCode::OK, "ok\n") }))
      .nest_service("/mcp", service),
  )
}

pub async fn run(options: Options) -> Result<()> {
  options.validate()?;
  let cancellation = CancellationToken::new();
  let app = router(&options, cancellation.clone())?;
  let listener = TcpListener::bind(options.listen)
    .await
    .with_context(|| format!("binding {}", options.listen))?;
  let address = listener.local_addr()?;
  eprintln!("dprint markdown-ja MCP server listening on http://{address}/mcp");
  axum::serve(listener, app)
    .with_graceful_shutdown(shutdown(cancellation))
    .await
    .context("serving MCP HTTP")?;
  Ok(())
}

async fn shutdown(cancellation: CancellationToken) {
  #[cfg(unix)]
  {
    use tokio::signal::unix::{SignalKind, signal};

    let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
      _ = tokio::signal::ctrl_c() => {}
      _ = terminate.recv() => {}
    }
  }
  #[cfg(not(unix))]
  let _ = tokio::signal::ctrl_c().await;
  cancellation.cancel();
}

#[cfg(test)]
mod tests {
  use super::*;

  fn defaults(markdown: &str) -> FormatMarkdownInput {
    FormatMarkdownInput {
      markdown: markdown.to_owned(),
      line_width: default_line_width(),
      text_wrap: default_text_wrap(),
      emphasis: default_emphasis(),
      strong: default_strong(),
    }
  }

  #[test]
  fn output_matches_core_and_reports_changes() {
    let input = "日本語English *text*";
    let expected = Formatter::default().format(input).unwrap();
    let output = format_markdown(defaults(input)).unwrap();
    assert_eq!(output.markdown, expected);
    assert!(output.changed);

    let output = format_markdown(defaults(&output.markdown)).unwrap();
    assert!(!output.changed);
  }

  #[test]
  fn options_match_core() {
    let input = FormatMarkdownInput {
      markdown: "alpha beta gamma delta epsilon\n\n_emphasis_ **strong**\n".to_owned(),
      line_width: 16,
      text_wrap: TextWrapOption::Always,
      emphasis: MarkerOption::Asterisks,
      strong: MarkerOption::Underscores,
    };
    let expected = Formatter::new(16, "always", "asterisks", "underscores")
      .unwrap()
      .format(&input.markdown)
      .unwrap()
      .into_owned();
    assert_eq!(format_markdown(input).unwrap().markdown, expected);
  }

  #[test]
  fn non_loopback_requires_host_and_limits_must_be_positive() {
    let mut options = Options::try_parse_from(["test", "--listen", "0.0.0.0:3000"]).unwrap();
    assert!(options.validate().is_err());
    options.allowed_host.push("markdown:3000".to_owned());
    assert!(options.validate().is_ok());
    options.max_concurrency = 0;
    assert!(options.validate().is_err());
  }
}
