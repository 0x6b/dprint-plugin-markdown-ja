use std::{
  fs,
  io::Write,
  process::{Command, Stdio},
};

fn run(args: &[&str], input: &[u8]) -> std::process::Output {
  run_in(args, input, None)
}

fn run_in(args: &[&str], input: &[u8], current_dir: Option<&std::path::Path>) -> std::process::Output {
  let mut command = Command::new(env!("CARGO_BIN_EXE_dprint-markdown-ja-formatter"));
  command
    .args(args)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  if let Some(current_dir) = current_dir {
    command.current_dir(current_dir);
  }
  let mut child = command.spawn().unwrap();
  child.stdin.take().unwrap().write_all(input).unwrap();
  child.wait_with_output().unwrap()
}

#[test]
fn stdin_and_check_status() {
  let result = run(&[], "日本語English *text*".as_bytes());
  assert!(result.status.success());
  assert_eq!(String::from_utf8(result.stdout).unwrap(), "日本語 English _text_\n");
  let result = run(&["--check", "-"], b"*text*");
  assert_eq!(result.status.code(), Some(1));
  assert!(result.stdout.is_empty());
  assert_eq!(run(&["--check"], b"_text_\n").status.code(), Some(0));
  assert_eq!(run(&[], &[0xff]).status.code(), Some(2));
  for args in [
    vec!["--bad"],
    vec!["--line-width", "0"],
    vec!["--text-wrap", "bad"],
    vec!["--strong"],
    vec!["-", "file.md"],
  ] {
    assert_eq!(run(&args, b"").status.code(), Some(2));
  }
  assert!(run(&["--help"], b"").status.success());
  let result = run(
    &[
      "--line-width",
      "16",
      "--text-wrap",
      "always",
      "--emphasis",
      "asterisks",
      "--strong",
      "underscores",
    ],
    b"alpha beta gamma delta epsilon\n\n_emphasis_ **strong**\n",
  );
  assert!(result.status.success());
  assert_eq!(
    result.stdout,
    b"alpha beta gamma\ndelta epsilon\n\n*emphasis*\n__strong__\n"
  );
}

#[test]
fn files_check_then_write_then_check() {
  let dir = std::env::temp_dir().join(format!("markdown-ja-cli-{}", std::process::id()));
  fs::create_dir(&dir).unwrap();
  let a = dir.join("a.md");
  let b = dir.join("b.md");
  fs::write(&a, "*first*").unwrap();
  fs::write(&b, "__second__").unwrap();
  let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
  assert_eq!(run(&["--check", a, b], b"").status.code(), Some(1));
  assert_eq!(fs::read_to_string(a).unwrap(), "*first*");
  assert!(run(&["--", a, b], b"").status.success());
  assert_eq!(fs::read_to_string(a).unwrap(), "_first_\n");
  assert_eq!(fs::read_to_string(b).unwrap(), "**second**\n");
  assert_eq!(run(&["--check", a, b], b"").status.code(), Some(0));
  fs::remove_dir_all(dir).unwrap();
}

#[test]
fn directories_route_supported_extensions_and_apply_excludes() {
  let dir = tempfile::tempdir().unwrap();
  for (path, contents) in [
    ("README.md", "*root*"),
    ("docs/guide.markdown", "*guide*"),
    ("config.json", "{\"enabled\":true,\"count\":2}"),
    ("settings.jsonc", "{// keep this comment\n\"enabled\":true}"),
    ("docs/generated.md", "*generated*"),
    ("node_modules/root.md", "*dependency*"),
    ("docs/node_modules/nested.md", "*nested dependency*"),
    ("notes.txt", "*plain text*"),
    ("data-lock.json", "*lock file*"),
  ] {
    let path = dir.path().join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
  }

  let result = run_in(&["--excludes", "docs/generated.md", "."], b"", Some(dir.path()));
  assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
  assert_eq!(fs::read_to_string(dir.path().join("README.md")).unwrap(), "_root_\n");
  assert_eq!(
    fs::read_to_string(dir.path().join("docs/guide.markdown")).unwrap(),
    "_guide_\n"
  );
  assert_eq!(
    fs::read_to_string(dir.path().join("config.json")).unwrap(),
    "{ \"enabled\": true, \"count\": 2 }\n"
  );
  assert_eq!(
    fs::read_to_string(dir.path().join("settings.jsonc")).unwrap(),
    "{ // keep this comment\n  \"enabled\": true\n}\n"
  );
  for path in [
    "docs/generated.md",
    "node_modules/root.md",
    "docs/node_modules/nested.md",
    "notes.txt",
    "data-lock.json",
  ] {
    assert!(fs::read_to_string(dir.path().join(path)).unwrap().starts_with('*'));
  }

  let lock = dir.path().join("explicit-lock.json");
  fs::write(&lock, "*explicit lock*").unwrap();
  let result = run_in(&[lock.to_str().unwrap()], b"", Some(dir.path()));
  assert!(result.status.success());
  assert_eq!(fs::read_to_string(lock).unwrap(), "*explicit lock*");
}

#[cfg(unix)]
#[test]
fn atomic_replacement_preserves_mode_and_symlink_and_skips_unchanged_files() {
  use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
  let dir = tempfile::tempdir().unwrap();
  let target = dir.path().join("target.md");
  let link = dir.path().join("link.md");
  fs::write(&target, "*text*").unwrap();
  fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
  symlink(&target, &link).unwrap();
  assert!(run(&[link.to_str().unwrap()], b"").status.success());
  assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
  assert_eq!(fs::read_to_string(&target).unwrap(), "_text_\n");
  let before = fs::metadata(&target).unwrap();
  assert_eq!(before.permissions().mode() & 0o777, 0o640);
  assert!(run(&[link.to_str().unwrap()], b"").status.success());
  let after = fs::metadata(&target).unwrap();
  assert_eq!(before.ino(), after.ino());
  assert_eq!(before.modified().unwrap(), after.modified().unwrap());
}
