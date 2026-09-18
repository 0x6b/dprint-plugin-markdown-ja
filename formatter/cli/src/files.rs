use anyhow::{Context, Result, bail};
use ignore::{WalkBuilder, gitignore::GitignoreBuilder};
use std::{
  borrow::Cow,
  collections::HashSet,
  fs,
  io::Write,
  path::{Path, PathBuf},
};

const DEFAULT_EXCLUDES: &[&str] = &["**/node_modules", "**/*-lock.json"];
const EXTENSIONS: &[&str] = &["md", "mkd", "mdwn", "mkdn", "mdown", "markdown", "json", "jsonc"];

pub(super) fn collect(paths: Vec<PathBuf>, excludes: &[String]) -> Result<Vec<PathBuf>> {
  let cwd = std::env::current_dir().context("resolving current directory")?;
  let mut builder = GitignoreBuilder::new(&cwd);
  for pattern in DEFAULT_EXCLUDES
    .iter()
    .copied()
    .chain(excludes.iter().map(String::as_str))
  {
    builder
      .add_line(None, pattern)
      .with_context(|| format!("invalid exclude pattern {pattern:?}"))?;
  }
  let matcher = builder.build().context("building exclude patterns")?;
  let mut files = Vec::new();
  let mut seen = HashSet::new();

  for path in paths {
    let metadata = fs::metadata(&path).with_context(|| format!("reading {}", path.display()))?;
    if metadata.is_file() {
      if !excluded(&matcher, &cwd, &path, false) {
        push_unique(path, &mut files, &mut seen)?;
      }
      continue;
    }
    if !metadata.is_dir() {
      bail!("{} is not a file or directory", path.display());
    }

    let (matcher, cwd) = (matcher.clone(), cwd.clone());
    let mut walker = WalkBuilder::new(&path);
    walker.standard_filters(false).filter_entry(move |entry| {
      !excluded(
        &matcher,
        &cwd,
        entry.path(),
        entry.file_type().is_some_and(|kind| kind.is_dir()),
      )
    });
    for entry in walker.build() {
      let entry = entry.with_context(|| format!("walking {}", path.display()))?;
      if entry.file_type().is_some_and(|kind| kind.is_file()) && supported(entry.path()) {
        push_unique(entry.into_path(), &mut files, &mut seen)?;
      }
    }
  }
  files.sort();
  Ok(files)
}

fn excluded(matcher: &ignore::gitignore::Gitignore, cwd: &Path, path: &Path, is_dir: bool) -> bool {
  let path = if path.is_absolute() {
    path
      .strip_prefix(cwd)
      .map(Cow::Borrowed)
      .unwrap_or_else(|_| Cow::Owned(path.components().skip(1).collect()))
  } else {
    Cow::Borrowed(path)
  };
  matcher.matched_path_or_any_parents(path, is_dir).is_ignore()
}

fn supported(path: &Path) -> bool {
  path
    .extension()
    .and_then(|extension| extension.to_str())
    .is_some_and(|extension| EXTENSIONS.contains(&extension))
}

fn push_unique(path: PathBuf, files: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) -> Result<()> {
  if seen.insert(fs::canonicalize(&path)?) {
    files.push(path);
  }
  Ok(())
}

pub(super) fn write_atomic(path: &Path, output: &[u8]) -> Result<()> {
  // Resolve symlinks so replacement updates the target, not the link.
  let target = fs::canonicalize(path)?;
  let mut temporary = tempfile::NamedTempFile::new_in(target.parent().context("file has no parent directory")?)?;
  temporary
    .as_file()
    .set_permissions(fs::metadata(&target)?.permissions())?;
  temporary.write_all(output)?;
  temporary.as_file().sync_all()?;
  temporary
    .persist(&target)
    .with_context(|| format!("writing {}", path.display()))?;
  Ok(())
}
