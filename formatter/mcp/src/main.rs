use clap::Parser;
use dprint_markdown_ja_formatter_mcp::{Options, run};

#[tokio::main]
async fn main() {
  if let Err(error) = run(Options::parse()).await {
    eprintln!("dprint-markdown-ja-formatter-mcp: {error:#}");
    std::process::exit(2);
  }
}
