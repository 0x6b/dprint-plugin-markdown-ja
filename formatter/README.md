# dprint-markdown-ja-formatter

Japanese-aware Markdown formatting as a native CLI and an Android-ready JNI library. This standalone project has no dependency on Entry or a dprint process, WASM runtime, network service, or consuming application's package name.

## Architecture and formatting contract

The implementation is split into three workspace packages. `core` owns the safe, reusable `Formatter` API and configuration validation. `cli` handles file discovery, Markdown/JSON routing, arguments, and I/O. `android` owns only JNI transport and the Android `cdylib`; it does not depend on the CLI's JSON, glob, or filesystem crates. Both adapters depend on core, which statically links the parent **dprint-plugin-markdown-ja v0.6.1** package. The repository revision pins the implementations together; the migration baseline is [this commit](https://github.com/0x6b/dprint-plugin-markdown-ja/commit/786296fde0c665b1bf4a1409bee1c649f50989c4). The root `rust-toolchain.toml` pins Rust **1.92.0** and root `Cargo.lock` pins transitive dependencies. Use `--locked` in builds. No plugin WASM feature is enabled.

Run the commands below from `formatter/`. Build outputs and the lockfile are shared with the parent workspace. Its default member remains the Wasm plugin. Use `--profile formatter-release` for native release builds: ordinary `--release` inherits the plugin's `panic=abort` and is deliberately rejected by the JNI adapter.

Defaults: line width **80**, text wrap **never**, emphasis **underscores**, strong **asterisks**, and table formatting disabled (`skipTableFormatting: true`). Other upstream defaults apply (including LF). Width must be 1..10000. Supported options are deliberately limited to these four rather than exposing an unversioned configuration JSON interface.

```rust
use dprint_markdown_ja_formatter_core::Formatter;
let formatter = Formatter::default();
let output = formatter.format("日本語English *text*")?;
assert_eq!(output, "日本語 English _text_\n");
# Ok::<(), anyhow::Error>(())
```

`format` returns `Result<Cow<str>>`: unchanged input is borrowed. JNI always returns text, never an upstream “unchanged” sentinel. Java object identity is not promised.

**Formatting follows the upstream plugin**, including code fences and enclosing lists/quotes. The core returns `dprint_plugin_markdown_ja::format_text` directly, with a host code-formatting callback returning `Ok(None)`. There is no byte-for-byte restoration: upstream may normalize fence delimiters, indentation, leading/trailing blank space and line endings, close an unclosed fence, and format prose inside containers. Markdown (`md`/`markdown`) fences are recursively formatted by upstream. No external Rust/JavaScript/YAML formatter is invoked. The compatibility target is stock dprint with only this pinned markdown-ja plugin and the same configuration, not a dprint setup with additional language plugins. This intentionally replaces the initial adapter's custom fence-preservation behavior.

## CLI

Install rustup and a platform C linker, then:

```sh
cargo build -p dprint-markdown-ja-formatter-cli --profile formatter-release --locked
printf '日本語English *text*' | ../target/formatter-release/dprint-markdown-ja-formatter
../target/formatter-release/dprint-markdown-ja-formatter README.md notes.md
../target/formatter-release/dprint-markdown-ja-formatter .
../target/formatter-release/dprint-markdown-ja-formatter --check README.md notes.md
../target/formatter-release/dprint-markdown-ja-formatter --excludes 'generated/**' .
../target/formatter-release/dprint-markdown-ja-formatter --text-wrap always --line-width 60 < input.md
../target/formatter-release/dprint-markdown-ja-formatter --help
```

No paths, or a single `-`, reads UTF-8 Markdown from stdin and writes stdout. File paths are updated in place; use `--` before names beginning with `-`. File format is selected by extension: `json` and `jsonc` use the statically linked dprint JSON plugin v0.23.0, while other explicit files retain the existing Markdown behavior. Directory paths are searched recursively for JSON, JSONC, and the same Markdown extensions as the dprint plugin (`md`, `mkd`, `mdwn`, `mkdn`, `mdown`, and `markdown`). Stdin cannot be mixed with paths and remains Markdown-only.

Directory discovery and explicit files exclude `**/node_modules` and `**/*-lock.json` by default. Repeat `--excludes GLOB` to add gitignore-style patterns; patterns are matched relative to the current directory and later negated patterns can re-include an earlier exclusion. This is the native equivalent of the standalone formatter's intended dprint defaults. The native binary contains both formatters, so it does not load the Wasm `plugins` list. It deliberately does not read `dprint.json` or `.gitignore`. The formatting options above apply only to Markdown; JSON and JSONC use the JSON plugin defaults.

`--check` writes no formatted text, reports changed paths on stderr, and exits **1** on differences, **0** if clean. Invalid arguments, patterns, UTF-8, formatting, or I/O errors exit **2** with a diagnostic.

Changed files are written to a temporary file in the same directory and atomically renamed over the target, preserving permissions and following symlinks. This needs directory write permission, replaces the inode (other hard links are not updated), and does not preserve ownership, ACLs, or extended attributes. Multi-file runs are not transactions; earlier successful writes remain if a later file fails. Avoid concurrent editors; files are not locked. Unchanged files are not rewritten.

## Java and Kotlin / host JNI

The Java 8-compatible facade owns the stable package `io.warpnine.markdownja`, based on the reverse domain name of `warpnine.io`; consumers do not rename it. The only JNI export is `JNI_OnLoad`, which uses `RegisterNatives` to register the private native method. There are no application-specific exported names or native handles to close.

```java
import io.warpnine.markdownja.MarkdownFormatter;
String formatted = MarkdownFormatter.format("日本語English *text*");
String wrapped = MarkdownFormatter.format(text, 60, MarkdownFormatter.TextWrap.ALWAYS,
    MarkdownFormatter.Marker.UNDERSCORES, MarkdownFormatter.Marker.ASTERISKS);
```

```kotlin
import io.warpnine.markdownja.MarkdownFormatter
val formatted = MarkdownFormatter.format("日本語English *text*")
```

For a desktop JVM, compile the facade and put the host shared library on `java.library.path` (Linux: `libdprint_markdown_ja_formatter.so`; other hosts require their own build). Example integration test with JDK 11+:

```sh
bash scripts/test-jvm.sh
# Internally: cargo build --locked --lib; javac --release 8; java -Xcheck:jni
```

JNI converts Java UTF-16 strictly: supplementary Unicode and embedded NUL survive, while unpaired surrogates and null arguments raise `IllegalArgumentException`. Invalid widths/options and formatter failures also raise useful `IllegalArgumentException`s. Rust panics are caught at both FFI entry points; formatting panics raise `RuntimeException`, and load failures reject the library. Existing JVM exceptions (for example allocation failures) are preserved. Keep Rust's unwind panic strategy; do not build with `panic=abort`. Abort, OOM in Rust, and stack exhaustion cannot be converted into Java exceptions. Calls are stateless and thread-safe. Run large documents on an Android worker thread; no input-size, time-budget, cancellation, or hostile-document isolation is provided.

## Android ARM64 AAR

Install an Android NDK (r28+ recommended), JDK 11+, Python 3, and rustup. No Gradle or Android SDK is needed to assemble this resource-free AAR:

```sh
export ANDROID_NDK_HOME=/absolute/path/to/android-ndk
bash scripts/build-aar.sh
# ../target/dprint-markdown-ja-formatter.aar
```

The script builds `aarch64-linux-android` release JNI for **arm64-v8a / API 21+**, sets 16 KiB ELF segment alignment, and packages `classes.jar`, the native library, manifest, R8 keep rules, and license notices. It supports Linux x86-64 and macOS NDK hosts. Other ABIs are not included. Use Android Gradle Plugin 8.5.1+ for modern 16 KiB APK native-library packaging. The AAR ZIP timestamps and NDK are not pinned; source/dependency reproducibility does not imply bit-identical binaries.

The packaging script explicitly runs the selected NDK's `llvm-strip --strip-unneeded` on the staging copy, rather than relying on AGP stripping it later. Cargo's original `.so` remains under `../target/aarch64-linux-android/formatter-release/` with its static symbols for symbolication (not full DWARF debug information). The AAR retains dynamic exports, relocations, and unwind tables. The formatter profile uses fat LTO, one codegen unit, size optimization `s`, and explicitly keeps `panic = "unwind"`.

Copy the AAR into your app's `libs/`, then in `build.gradle.kts`:

```kotlin
dependencies {
    implementation(files("libs/dprint-markdown-ja-formatter.aar"))
}
```

The facade calls `System.loadLibrary` on first use. The AAR carries consumer keep rules required by dynamic registration. If packaging the Java source and `.so` manually, include `android/consumer-rules.pro` and place the library under `src/main/jniLibs/arm64-v8a/`. Android cross-build and AAR ELF checks have been verified with NDK r28c (28.2.13676358). **The optimized AAR still needs minified Entry APK / real ARM64 device validation, including API 21 and 16 KiB devices.** Host JVM tests do not establish Android runtime compatibility or on-device performance.

### Size measurements and decisions

Measured before workspace migration on Linux x86-64 with Rust 1.92.0, NDK 28.2.13676358, OpenJDK 17, and the standalone project's lockfile. All sizes below are bytes; `.so` columns are uncompressed ELF sizes. AARs include the Java facade and license notices. ZIP/JAR timestamps can change the compressed size by a few bytes between runs. The workspace retains the parent plugin's existing dependency versions, so migration measurements are separate.

| Configuration                                                                 | Cargo `.so` before NDK strip | Packaged `.so` |       AAR |
| ----------------------------------------------------------------------------- | ---------------------------: | -------------: | --------: |
| Original: LTO, opt-level 3, default 16 codegen units, debuginfo stripped only |                    4,001,088 |      4,001,088 | 1,276,216 |
| Original + NDK `--strip-unneeded`                                             |                    4,001,088 |      2,791,432 | 1,053,056 |
| Above + codegen-units 1                                                       |                    3,113,736 |      2,389,624 |   949,965 |
| Above + opt-level `s`                                                         |                    3,243,608 |      2,315,896 |   885,638 |
| Alternative: codegen-units 1 + opt-level `z`                                  |                    3,661,728 |      2,282,008 |   894,180 |
| Selected: codegen-units 1 + `s`, custom fence restoration removed             |                    3,235,000 |      2,311,112 |   883,689 |
| Selected + experimental linker `--icf=safe`                                   |                    3,235,000 |      2,311,112 |   883,688 |
| Workspace migration, selected profile, parent plugin lockfile                 |                    3,193,104 |      2,281,968 |   868,304 |

Adopted strip, one codegen unit, and `s`: the selected packaged `.so` is 42.2% smaller and the AAR 30.8% smaller than the original. Removing custom fence restoration independently saves another 4,784 packaged ELF bytes relative to `s` with restoration; it is primarily a compatibility simplification, not a large dependency removal (upstream still uses pulldown-cmark). Rejected `z`: it saves 33,888 ELF bytes versus `s` before the simplification but makes the AAR 8,542 bytes larger and disables loop vectorization. Rejected `--icf=safe`: no ELF size benefit; the tiny AAR difference is packaging noise. No `panic=abort`, custom standard library, unsafe identical-code folding, packed relocations/RELR requiring newer Android loaders, or runtime decompression was added. Device latency has not been benchmarked; recheck it in Entry before adopting the size-oriented profile for a release.

Every measured variant retained only `JNI_OnLoad` as a defined dynamic export, four 16 KiB LOAD segments, and dependencies `libc.so` / `libdl.so`. The final AAR's 23 allocated sections match the pre-strip ELF byte-for-byte, including `.dynsym`, relocations, `.eh_frame`, and `.gcc_except_table`; its Android ident note reports API 21. Re-run the packaged-artifact check after building:

```sh
python3 scripts/check-aar.py "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
```

For reproducing profile alternatives, use `CARGO_PROFILE_FORMATTER_RELEASE_CODEGEN_UNITS=16` and/or `CARGO_PROFILE_FORMATTER_RELEASE_OPT_LEVEL=3` (or `z`) when running `scripts/build-aar.sh`. The script prints pre-strip ELF, packaged ELF, and AAR sizes. Pre-simplification measurements refer to the initial implementation; profile overrides alone do not restore its behavior.

## Verification and licenses

```sh
cargo fmt --all -- --check
cargo clippy --locked -p dprint-markdown-ja-formatter-core -p dprint-markdown-ja-formatter-cli -p dprint-markdown-ja-formatter-android --all-targets -- -D warnings
cargo test --locked --workspace
bash scripts/test-jvm.sh
```

Tests distinguish Japanese/Latin spacing, wrapping and marker options, borrowed unchanged output, upstream fence/container normalization, CLI Markdown/JSON routing, check statuses and errors, Java Unicode and invalid arguments, and 4,000 concurrent/repeated JNI calls with `-Xcheck:jni`.

GitHub Actions runs these checks for every pull request and push. After they pass, a separate job installs NDK r28c, builds and inspects the AAR, and uploads it as a workflow artifact. The Amp orb setup intentionally omits the NDK; local AAR builds require `ANDROID_NDK_HOME` as described above.

Migration verification also passed workspace tests (including 75 plugin specs), workspace rustfmt, formatter Clippy with warnings denied, native optimized-profile tests, debug and optimized real JVM tests, and the Android AAR ELF checks. Workspace-wide Clippy still reports the pre-existing `unnecessary_get_then_check` in the plugin test at `src/configuration/builder.rs:210`. The rebuilt Wasm has the identical SHA-256 below; the stock comparison was repeated against it successfully. No Android device tests were run in this orb.

The optional stock comparison test was also executed: stock dprint **0.57.4** matched all nine representative inputs (fences, CRLF, blank/trailing whitespace, nested list/quote prose, unclosed fence, Japanese spacing, and ignore-file). The Wasm was built using Rust 1.92.0 from the pinned v0.6.1 commit above, with that repository's lockfile and `cargo build --locked --release --target wasm32-unknown-unknown --features wasm`; its SHA-256 was `e7fe672ff0792d7748e54e816a54b910e3cb543f6a70b0c4283dfd94adb98f00`. This is a representative comparison, not proof of equivalence on all Markdown or dprint's multi-pass stabilization behavior. To repeat it with your local binaries:

```sh
DPRINT_BIN=/absolute/path/to/dprint-0.57.4 \
MARKDOWN_JA_WASM=/absolute/path/to/dprint_plugin_markdown_ja.wasm \
cargo test --locked -p dprint-markdown-ja-formatter-core --test core stock_dprint_parity -- --ignored
```

These adapters are MIT licensed. The upstream plugin is MIT, copyright **2024 0x6b** and **2020–2023 David Sherret**. Full dependency notices are generated from locked Cargo sources, not checked into Git. The AAR build automatically generates Android-only notices in `../target/THIRD_PARTY_NOTICES.md` and bundles them alongside `LICENSE`; generation failure stops packaging. For standalone CLI distribution, run `python3 scripts/license-notices.py dprint-markdown-ja-formatter-cli`, review the generated notices, and redistribute them and `LICENSE` alongside the binary. The generator requires Python 3 and covers Linux x86-64 and Android ARM64 (including build dependencies); regenerate/extend the platform set when distributing other targets.
