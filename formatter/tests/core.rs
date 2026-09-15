use dprint_markdown_ja_formatter::Formatter;
use std::borrow::Cow;

#[test]
fn japanese_spacing_and_defaults() {
  assert_eq!(
    Formatter::default()
      .format("日本語English日本語\nLatin words\n\n*emphasis* __strong__\n")
      .unwrap(),
    "日本語 English 日本語 Latin words\n\n_emphasis_ **strong**\n"
  );
}

#[test]
fn configured_markers_and_wrapping() {
  let f = Formatter::new(16, "always", "asterisks", "underscores").unwrap();
  assert_eq!(
    f.format("alpha beta gamma delta epsilon\n\n_emphasis_ **strong**\n")
      .unwrap(),
    "alpha beta gamma\ndelta epsilon\n\n*emphasis*\n__strong__\n"
  );
  assert_eq!(
    Formatter::new(16, "never", "underscores", "asterisks")
      .unwrap()
      .format("alpha beta gamma delta epsilon\n")
      .unwrap(),
    "alpha beta gamma delta epsilon\n"
  );
  assert_eq!(
    Formatter::new(80, "maintain", "underscores", "asterisks")
      .unwrap()
      .format("alpha beta\ngamma delta\n")
      .unwrap(),
    "alpha beta\ngamma delta\n"
  );
}

#[test]
fn unchanged_is_borrowed_and_ignore_file_is_honored() {
  for input in [
    "日本語 English\n",
    "<!-- dprint-ignore-file -->\n*raw*日本語English",
    "",
  ] {
    assert!(matches!(Formatter::default().format(input).unwrap(), Cow::Borrowed(_)));
  }
}

// Expectations independently checked against stock dprint 0.57.4 + the pinned
// markdown-ja Wasm. In particular, a no-op host callback does NOT preserve code.
const NORMALIZATION_CASES: &[(&str, &str)] = &[
  (
    "```rust\n\n  fn  main( ) { }  \n\n```",
    "```rust\nfn  main( ) { }\n```\n",
  ),
  ("~~~md\n*raw*日本語English  \n~~~", "```md\n*raw*日本語 English\n```\n"),
  ("```\n\n  x \t\n```", "```\nx\n```\n"),
  ("```js\r\n\r\n  x  \r\n```", "```js\nx\n```\n"),
  (
    "> *raw*\n>\n> ```md\n> 日本語English  \n> ```",
    "> _raw_\n>\n> ```md\n> 日本語 English\n> ```\n",
  ),
  (
    "- *raw*\n\n  ```md\n  日本語English  \n  ```",
    "- _raw_\n\n  ```md\n  日本語 English\n  ```\n",
  ),
  (
    "*before*\n\n```md\n*unclosed*  ",
    "_before_\n\n```md\n_unclosed_\n```\n",
  ),
  ("日本語English *text* __bold__", "日本語 English _text_ **bold**\n"),
  (
    "<!-- dprint-ignore-file -->\n*raw*日本語English",
    "<!-- dprint-ignore-file -->\n*raw*日本語English",
  ),
];

#[test]
fn fences_and_nested_containers_use_upstream_normalization() {
  for &(input, expected) in NORMALIZATION_CASES {
    let actual = Formatter::default().format(input).unwrap();
    assert_eq!(actual, expected);
    assert!(matches!(
      Formatter::default().format(&actual).unwrap(),
      Cow::Borrowed(_)
    ));
  }
}

#[test]
#[ignore = "requires DPRINT_BIN (0.57.4) and MARKDOWN_JA_WASM (pinned v0.6.1)"]
fn stock_dprint_parity() {
  use std::{
    io::Write,
    process::{Command, Stdio},
  };
  let dprint = std::env::var_os("DPRINT_BIN").expect("set DPRINT_BIN");
  let wasm = std::env::var_os("MARKDOWN_JA_WASM").expect("set MARKDOWN_JA_WASM");
  let version = Command::new(&dprint).arg("--version").output().unwrap();
  assert!(version.status.success());
  assert_eq!(String::from_utf8(version.stdout).unwrap().trim(), "dprint 0.57.4");
  let mut config = tempfile::NamedTempFile::new().unwrap();
  config
    .write_all(
      br#"{"markdownJa":{"lineWidth":80,"textWrap":"never","emphasisKind":"underscores","strongKind":"asterisks"}}"#,
    )
    .unwrap();
  for &(input, expected) in NORMALIZATION_CASES {
    let mut child = Command::new(&dprint)
      .args(["fmt", "--config"])
      .arg(config.path())
      .args(["--stdin", "case.md", "--plugins"])
      .arg(&wasm)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
    assert_eq!(Formatter::default().format(input).unwrap(), expected);
  }
}

#[test]
fn validates_every_option_and_width_boundary() {
  for width in [i32::MIN, 0, 10001, i32::MAX] {
    assert!(Formatter::new(width, "never", "underscores", "asterisks").is_err());
  }
  for width in [1, 10000] {
    assert!(Formatter::new(width, "never", "underscores", "asterisks").is_ok());
  }
  for (wrap, emphasis, strong) in [
    ("invalid", "underscores", "asterisks"),
    ("never", "bad", "asterisks"),
    ("never", "underscores", "bad"),
  ] {
    assert!(Formatter::new(80, wrap, emphasis, strong).is_err());
  }
}
