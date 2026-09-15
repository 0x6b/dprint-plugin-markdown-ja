use anyhow::{Context, Result, bail};
use dprint_markdown_ja_formatter::Formatter;
use std::{
  ffi::OsString,
  fs,
  io::{self, Read, Write},
  process::ExitCode,
};

const HELP: &str = "dprint-markdown-ja-formatter [OPTIONS] [FILE ...]\n\nNo files (or a single -): UTF-8 stdin to stdout. Files are updated in place.\n  --check                 Do not write; exit 1 if formatting differs\n  --line-width N          1..10000 (default 80)\n  --text-wrap MODE        never (default), maintain, always\n  --emphasis KIND         underscores (default), asterisks\n  --strong KIND           asterisks (default), underscores\n  --                      End options\n  -h, --help              Show help\nExit status: 0 success, 1 check differences, 2 error.\n";

fn run() -> Result<u8> {
  let mut args = std::env::args_os().skip(1);
  let (mut width, mut wrap, mut emphasis, mut strong) =
    (80, "never".to_owned(), "underscores".to_owned(), "asterisks".to_owned());
  let mut check = false;
  let mut files = Vec::<OsString>::new();
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
      Some(flag @ ("--line-width" | "--text-wrap" | "--emphasis" | "--strong")) => {
        let value = args
          .next()
          .context(format!("{flag} requires a value"))?
          .into_string()
          .map_err(|_| anyhow::anyhow!("option value must be UTF-8"))?;
        match flag {
          "--line-width" => width = value.parse().context("line width must be an integer")?,
          "--text-wrap" => wrap = value,
          "--emphasis" => emphasis = value,
          _ => strong = value,
        }
      }
      Some(s) if s.starts_with('-') && s != "-" => bail!("unknown option {s}; use --help"),
      _ => files.push(arg),
    }
  }
  let formatter = Formatter::new(width, &wrap, &emphasis, &strong)?;
  let stdin = files.is_empty() || (files.len() == 1 && files[0] == "-");
  if !stdin && files.iter().any(|f| f == "-") {
    bail!("stdin (-) cannot be combined with files");
  }
  let mut changed = false;
  if stdin {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).context("reading UTF-8 stdin")?;
    let output = formatter.format(&input)?;
    changed = output != input;
    if !check {
      io::stdout().lock().write_all(output.as_bytes())?;
    }
  } else {
    for file in files {
      let path = std::path::Path::new(&file);
      let input = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
      let output = formatter
        .format(&input)
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
