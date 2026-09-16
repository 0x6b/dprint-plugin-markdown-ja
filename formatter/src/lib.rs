//! Shared, safe formatting API. Adapters only handle transport and errors.
#[cfg(panic = "abort")]
compile_error!("JNI requires panic=unwind; use --profile formatter-release instead of --release");

use std::borrow::Cow;

use anyhow::{Result, ensure};
use dprint_plugin_markdown_ja::configuration::{Configuration, ConfigurationBuilder};
pub use dprint_plugin_markdown_ja::configuration::{EmphasisKind, StrongKind, TextWrap};

mod jni;

/// Reusable immutable formatter; calls have no shared mutable state.
pub struct Formatter(Configuration);

impl Default for Formatter {
  fn default() -> Self {
    Self(
      ConfigurationBuilder::new()
        .line_width(80)
        .text_wrap(TextWrap::Never)
        .emphasis_kind(EmphasisKind::Underscores)
        .strong_kind(StrongKind::Asterisks)
        .skip_table_formatting(true)
        .build(),
    )
  }
}

impl Formatter {
  /// Width is limited to 1..=10000 to reject accidental/hostile extreme values.
  /// Option names match dprint, and are shared by the CLI and JNI wire API.
  pub fn new(width: i32, wrap: &str, emphasis: &str, strong: &str) -> Result<Self> {
    ensure!((1..=10_000).contains(&width), "line width must be between 1 and 10000");
    let wrap = wrap
      .parse::<TextWrap>()
      .map_err(|_| anyhow::anyhow!("text wrap must be never, maintain, or always"))?;
    let emphasis = emphasis
      .parse::<EmphasisKind>()
      .map_err(|_| anyhow::anyhow!("emphasis must be underscores or asterisks"))?;
    let strong = strong
      .parse::<StrongKind>()
      .map_err(|_| anyhow::anyhow!("strong must be underscores or asterisks"))?;
    Ok(Self(
      ConfigurationBuilder::new()
        .line_width(width as u32)
        .text_wrap(wrap)
        .emphasis_kind(emphasis)
        .strong_kind(strong)
        .skip_table_formatting(true)
        .build(),
    ))
  }

  /// Returns borrowed input when unchanged, including ignore-file documents.
  /// Uses upstream normalization, without an external code-language formatter.
  pub fn format<'a>(&self, input: &'a str) -> Result<Cow<'a, str>> {
    Ok(
      match dprint_plugin_markdown_ja::format_text(input, &self.0, |_, _, _| Ok(None))? {
        Some(output) => Cow::Owned(output),
        None => Cow::Borrowed(input),
      },
    )
  }
}
