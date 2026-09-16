use anyhow::{Context, Result, bail};
use dprint_markdown_ja_formatter::Formatter;
use dprint_plugin_json::configuration::{
  Configuration as JsonConfiguration, ConfigurationBuilder as JsonConfigurationBuilder,
};
use ignore::{WalkBuilder, gitignore::GitignoreBuilder};
use std::{
  borrow::Cow,
  collections::HashSet,
  ffi::OsString,
  fs,
  io::{self, Read, Write},
  path::{Path, PathBuf},
  process::ExitCode,
};

const HELP: &str = "dprint-markdown-ja-formatter [OPTIONS] [PATH ...]\n\nNo paths (or a single -): UTF-8 Markdown stdin to stdout. Files are updated in place.\nDirectories are searched recursively for Markdown, JSON, and JSONC files.\n  --check                 Do not write; exit 1 if formatting differs\n  --line-width N          Markdown: 1..10000 (default 80)\n  --text-wrap MODE        Markdown: never (default), maintain, always\n  --emphasis KIND         Markdown: underscores (default), asterisks\n  --strong KIND           Markdown: asterisks (default), underscores\n  --excludes GLOB         Add a gitignore-style exclusion (repeatable)\n                          Defaults: **/node_modules, **/*-lock.json\n  --                      End options\n  -h, --help              Show help\nExit status: 0 success, 1 check differences, 2 error.\n";

const DEFAULT_EXCLUDES: &[&str] = &["**/node_modules", "**/*-lock.json"];
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "mkd", "mdwn", "mkdn", "mdown", "markdown"];

fn is_excluded(matcher: &ignore::gitignore::Gitignore, current_dir: &Path, path: &Path, is_dir: bool) -> bool {
  let relative = if path.is_absolute() {
    match path.strip_prefix(current_dir) {
      Ok(path) => Cow::Borrowed(path),
      Err(_) => Cow::Owned(path.components().skip(1).collect()),
    }
  } else {
    Cow::Borrowed(path)
  };
  matcher.matched_path_or_any_parents(relative, is_dir).is_ignore()
}

fn collect_files(paths: Vec<OsString>, excludes: &[String]) -> Result<Vec<PathBuf>> {
  let current_dir = std::env::current_dir().context("resolving current directory")?;
  let mut matcher = GitignoreBuilder::new(&current_dir);
  for pattern in DEFAULT_EXCLUDES
    .iter()
    .copied()
    .chain(excludes.iter().map(String::as_str))
  {
    matcher
      .add_line(None, pattern)
      .with_context(|| format!("invalid exclude pattern {pattern:?}"))?;
  }
  let matcher = matcher.build().context("building exclude patterns")?;
  let mut files = Vec::new();
  let mut seen = HashSet::new();

  for path in paths {
    let path = PathBuf::from(path);
    let metadata = fs::metadata(&path).with_context(|| format!("reading {}", path.display()))?;
    if metadata.is_file() {
      if !is_excluded(&matcher, &current_dir, &path, false) {
        let canonical = fs::canonicalize(&path)?;
        if seen.insert(canonical) {
          files.push(path);
        }
      }
      continue;
    }
    if !metadata.is_dir() {
      bail!("{} is not a file or directory", path.display());
    }

    let mut walker = WalkBuilder::new(&path);
    let walker_matcher = matcher.clone();
    let walker_current_dir = current_dir.clone();
    walker.standard_filters(false).filter_entry(move |entry| {
      !is_excluded(
        &walker_matcher,
        &walker_current_dir,
        entry.path(),
        entry.file_type().is_some_and(|kind| kind.is_dir()),
      )
    });
    for entry in walker.build() {
      let entry = entry.with_context(|| format!("walking {}", path.display()))?;
      if !entry.file_type().is_some_and(|kind| kind.is_file()) || !is_supported(entry.path()) {
        continue;
      }
      let canonical = fs::canonicalize(entry.path())?;
      if seen.insert(canonical) {
        files.push(entry.into_path());
      }
    }
  }
  files.sort();
  Ok(files)
}

fn is_markdown(path: &Path) -> bool {
  path
    .extension()
    .and_then(|extension| extension.to_str())
    .is_some_and(|extension| MARKDOWN_EXTENSIONS.contains(&extension))
}

fn is_json(path: &Path) -> bool {
  path
    .extension()
    .and_then(|extension| extension.to_str())
    .is_some_and(|extension| matches!(extension, "json" | "jsonc"))
}

fn is_supported(path: &Path) -> bool {
  is_markdown(path) || is_json(path)
}

fn format_file<'a>(
  path: &Path,
  input: &'a str,
  markdown: &Formatter,
  json: &JsonConfiguration,
) -> Result<Cow<'a, str>> {
  if is_json(path) {
    Ok(match dprint_plugin_json::format_text(path, input, json)? {
      Some(output) => Cow::Owned(output),
      None => Cow::Borrowed(input),
    })
  } else {
    markdown.format(input)
  }
}

fn run() -> Result<u8> {
  let mut args = std::env::args_os().skip(1);
  let (mut width, mut wrap, mut emphasis, mut strong) =
    (80, "never".to_owned(), "underscores".to_owned(), "asterisks".to_owned());
  let mut check = false;
  let mut files = Vec::<OsString>::new();
  let mut excludes = Vec::new();
  while let Some(arg) = args.next() {
    match arg.to_str() {
      Some("-h" | "--help") => {
        print!("{HELP}");
        return Ok(0);
      }
      Some("--") => {
        files.extend(args);
        break;
      }
      Some("--check") => check = true,
      Some(flag @ ("--line-width" | "--text-wrap" | "--emphasis" | "--strong" | "--excludes")) => {
        let value = args
          .next()
          .context(format!("{flag} requires a value"))?
          .into_string()
          .map_err(|_| anyhow::anyhow!("option value must be UTF-8"))?;
        match flag {
          "--line-width" => width = value.parse().context("line width must be an integer")?,
          "--text-wrap" => wrap = value,
          "--emphasis" => emphasis = value,
          "--strong" => strong = value,
          _ => excludes.push(value),
        }
      }
      Some(s) if s.starts_with('-') && s != "-" => bail!("unknown option {s}; use --help"),
      _ => files.push(arg),
    }
  }
  let markdown_formatter = Formatter::new(width, &wrap, &emphasis, &strong)?;
  let json_formatter = JsonConfigurationBuilder::new().build();
  let stdin = files.is_empty() || (files.len() == 1 && files[0] == "-");
  if !stdin && files.iter().any(|f| f == "-") {
    bail!("stdin (-) cannot be combined with files");
  }
  let files = if stdin {
    Vec::new()
  } else {
    collect_files(files, &excludes)?
  };
  let mut changed = false;
  if stdin {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).context("reading UTF-8 stdin")?;
    let output = markdown_formatter.format(&input)?;
    changed = output != input;
    if !check {
      io::stdout().lock().write_all(output.as_bytes())?;
    }
  } else {
    for file in files {
      let path = std::path::Path::new(&file);
      let input = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
      let output = format_file(path, &input, &markdown_formatter, &json_formatter)
        .with_context(|| format!("formatting {}", path.display()))?;
      if output != input {
        changed = true;
        if check {
          eprintln!("{}", path.display());
        } else {
          // Resolve symlinks so we replace the target, not the link.
          // A same-directory rename avoids truncation on write failure.
          let target = fs::canonicalize(path)?;
          let mut temporary =
            tempfile::NamedTempFile::new_in(target.parent().context("file has no parent directory")?)?;
          temporary
            .as_file()
            .set_permissions(fs::metadata(&target)?.permissions())?;
          temporary.write_all(output.as_bytes())?;
          temporary.as_file().sync_all()?;
          temporary
            .persist(&target)
            .with_context(|| format!("writing {}", path.display()))?;
        }
      }
    }
  }
  Ok(u8::from(check && changed))
}

fn main() -> ExitCode {
  match run() {
    Ok(code) => ExitCode::from(code),
    Err(error) => {
      eprintln!("dprint-markdown-ja-formatter: {error:#}");
      ExitCode::from(2)
    }
  }
}
