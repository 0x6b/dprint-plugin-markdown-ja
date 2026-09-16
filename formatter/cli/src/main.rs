mod files;

use anyhow::{Context, Result, bail};
use clap::Parser;
use dprint_markdown_ja_formatter_core::Formatter;
use dprint_plugin_json::configuration::{
  Configuration as JsonConfiguration, ConfigurationBuilder as JsonConfigurationBuilder,
};
use std::{
  borrow::Cow,
  fs,
  io::{self, Read, Write},
  path::{Path, PathBuf},
  process::ExitCode,
};

#[derive(Parser)]
#[command(
  about = "Format Markdown, JSON, and JSONC files",
  after_help = "No paths (or a single -) reads UTF-8 Markdown from stdin.\nExit status: 0 success, 1 check differences, 2 error."
)]
struct Cli {
  /// Do not write; exit 1 if formatting differs
  #[arg(long)]
  check: bool,

  /// Markdown line width (1..10000)
  #[arg(long, default_value_t = 80)]
  line_width: i32,

  /// Markdown text wrapping
  #[arg(long, default_value = "never", value_parser = ["never", "maintain", "always"])]
  text_wrap: String,

  /// Markdown emphasis marker
  #[arg(long, default_value = "underscores", value_parser = ["underscores", "asterisks"])]
  emphasis: String,

  /// Markdown strong marker
  #[arg(long, default_value = "asterisks", value_parser = ["asterisks", "underscores"])]
  strong: String,

  /// Add a gitignore-style exclusion (repeatable)
  #[arg(long, value_name = "GLOB")]
  excludes: Vec<String>,

  /// Files or directories; use -- before paths beginning with -
  #[arg(value_name = "PATH")]
  paths: Vec<PathBuf>,
}

struct Formatters {
  markdown: Formatter,
  json: JsonConfiguration,
}

impl Formatters {
  fn new(cli: &Cli) -> Result<Self> {
    Ok(Self {
      markdown: Formatter::new(cli.line_width, &cli.text_wrap, &cli.emphasis, &cli.strong)?,
      json: JsonConfigurationBuilder::new().build(),
    })
  }

  fn format<'a>(&self, path: &Path, input: &'a str) -> Result<Cow<'a, str>> {
    if is_json(path) {
      Ok(match dprint_plugin_json::format_text(path, input, &self.json)? {
        Some(output) => Cow::Owned(output),
        None => Cow::Borrowed(input),
      })
    } else {
      self.markdown.format(input)
    }
  }
}

fn is_json(path: &Path) -> bool {
  matches!(
    path.extension().and_then(|extension| extension.to_str()),
    Some("json" | "jsonc")
  )
}

fn format_stdin(formatter: &Formatter, check: bool) -> Result<bool> {
  let mut input = String::new();
  io::stdin().read_to_string(&mut input).context("reading UTF-8 stdin")?;
  let output = formatter.format(&input)?;
  let changed = output != input;
  if !check {
    io::stdout().lock().write_all(output.as_bytes())?;
  }
  Ok(changed)
}

fn format_file(path: &Path, formatters: &Formatters, check: bool) -> Result<bool> {
  let input = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
  let output = formatters
    .format(path, &input)
    .with_context(|| format!("formatting {}", path.display()))?;
  if output == input {
    return Ok(false);
  }
  if check {
    eprintln!("{}", path.display());
  } else {
    files::write_atomic(path, output.as_bytes())?;
  }
  Ok(true)
}

fn run(cli: Cli) -> Result<u8> {
  let formatters = Formatters::new(&cli)?;
  let stdin = cli.paths.is_empty() || cli.paths == [Path::new("-")];
  let changed = if stdin {
    format_stdin(&formatters.markdown, cli.check)?
  } else {
    if cli.paths.iter().any(|path| path == Path::new("-")) {
      bail!("stdin (-) cannot be combined with files");
    }
    let mut changed = false;
    for path in files::collect(cli.paths, &cli.excludes)? {
      changed |= format_file(&path, &formatters, cli.check)?;
    }
    changed
  };
  Ok(u8::from(cli.check && changed))
}

fn main() -> ExitCode {
  match run(Cli::parse()) {
    Ok(code) => ExitCode::from(code),
    Err(error) => {
      eprintln!("dprint-markdown-ja-formatter: {error:#}");
      ExitCode::from(2)
    }
  }
}
