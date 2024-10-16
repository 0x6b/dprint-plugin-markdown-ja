# dprint-plugin-markdown-ja

Opinionated and hacky Markdown formatter for [dprint](https://dprint.dev/) that follows Japanese-specific rules, based on the [dprint/dprint-plugin-markdown v0.17.8](https://github.com/dprint/dprint-plugin-markdown/releases/tag/0.17.8).

## Feature

The plugin will insert a space between a Japanese character and a non-Japanese character, except in the following opinionated rules:

- The previous character is Japanese, and the current character is one of ``()*/:_`~``.
- The previous character is one of ``()*/_`~``, and the current character is Japanese.
- Either the previous or current character is a Japanese symbol or punctuation mark.

See [Text_Japanese.txt](tests/specs/Text/Text_Japanese.txt) for how the plugin will work.

Please note that inserting a space in such a way is a controversial topic. See [Do not insert whitespaces between latin and cj letters · Issue #6385 · prettier/prettier](https://github.com/prettier/prettier/issues/6385) for the discussion in the context of Prettier.

## How It Works

The plugin will determine whether a character is **Japanese** or **Japanese symbols and punctuation** based on the Unicode code points. See the following sections.

### Japanese Characters

| Description                                                                              | Range                                        |
| ---------------------------------------------------------------------------------------- | -------------------------------------------- |
| [Hiragana](https://www.unicode.org/charts/PDF/U3040.pdf)                                 | `\u{3041}..=\u{3096}`, `\u{3099}..=\u{309F}` |
| [Kana Extended-A](https://www.unicode.org/charts/PDF/U1B100.pdf)                         | `\u{1B100}..=\u{1B12F}`                      |
| [Kana Extended-B](https://www.unicode.org/charts/PDF/U1AFF0.pdf)                         | `\u{1AFF0}..=\u{1AFFF}`                      |
| [Kana Supplement](https://www.unicode.org/charts/PDF/U1B000.pdf)                         | `\u{1B000}..=\u{1B0FF}`                      |
| [Small Kana Extension](https://www.unicode.org/charts/PDF/U1B130.pdf)                    | `\u{1B130}..=\u{1B16F}`                      |
| [Katakana](https://www.unicode.org/charts/PDF/U30A0.pdf)                                 | `\u{30A0}..=\u{30FF}`                        |
| [CJK Unified Ideographs](https://www.unicode.org/charts/PDF/U4E00.pdf)                   | `\u{4E00}..=\u{9FFF}`                        |
| [CJK Unified Ideographs Extension A](https://www.unicode.org/charts/PDF/U3400.pdf)       | `\u{3400}..=\u{4DBF}`                        |
| [CJK Unified Ideographs Extension B](https://www.unicode.org/charts/PDF/U20000.pdf)      | `\u{20000}..=\u{2A6DF}`                      |
| [CJK Unified Ideographs Extension C](https://www.unicode.org/charts/PDF/U2A700.pdf)      | `\u{2A700}..=\u{2B739}`                      |
| [CJK Unified Ideographs Extension D](https://www.unicode.org/charts/PDF/U2B740.pdf)      | `\u{2B740}..=\u{2B81D}`                      |
| [CJK Unified Ideographs Extension E](https://www.unicode.org/charts/PDF/U2B820.pdf)      | `\u{2B820}..=\u{2CEA1}`                      |
| [CJK Unified Ideographs Extension F](https://www.unicode.org/charts/PDF/U2CEB0.pdf)      | `\u{2CEB0}..=\u{2EBE0}`                      |
| [CJK Unified Ideographs Extension G](https://www.unicode.org/charts/PDF/U30000.pdf)      | `\u{30000}..=\u{3134A}`                      |
| [CJK Unified Ideographs Extension H](https://www.unicode.org/charts/PDF/U31350.pdf)      | `\u{31350}..=\u{323AF}`                      |
| [CJK Unified Ideographs Extension I](https://www.unicode.org/charts/PDF/U2EBF0.pdf)      | `\u{2EBF0}..=\u{2EE5D}`                      |
| [CJK Compatibility Ideographs](https://www.unicode.org/charts/PDF/UF900.pdf)             | `\u{F900}..=\u{FAFF}`                        |
| [CJK Compatibility Ideographs Supplement](https://www.unicode.org/charts/PDF/U2F800.pdf) | `\u{2F800}..=\u{2FA1F}`                      |
| Symbols and punctuation (see table below)                                                |                                              |

### Symbols and Punctuation

| Description                                                                   | Range                 |
| ----------------------------------------------------------------------------- | --------------------- |
| [CJK Symbols and Punctuation](https://www.unicode.org/charts/PDF/U3000.pdf)   | `\u{3000}..=\u{303F}` |
| [Halfwidth and Fullwidth Forms](https://www.unicode.org/charts/PDF/UFF00.pdf) | `\u{FF01}..=\u{FF60}` |
| [Katakana Middle Dot](https://www.unicode.org/charts/PDF/U30A0.pdf)           | `\u{30fb}`            |

## How to Build a WASM File for `dprint`

To build a `.wasm` file, you have to install `wasm32-unknown-unknown` target for Rust. This can be done using the following command:

```console
$ rustup target add wasm32-unknown-unknown
```

Then, you can build the `.wasm` file using the following command:

```console
$ cargo build --release --target=wasm32-unknown-unknown --features="wasm"
```

## How to Use

After the build, the `.wasm` file will be located at `target/wasm32-unknown-unknown/release/dprint_plugin_markdown_ja.wasm`. You can copy this file to the directory of yor choice and use it with `dprint`.

There are no additional configuration options specific to this plugin, but the configuration key is `markdownJa` instead of `markdown`. See [Markdown Plugin](https://dprint.dev/plugins/markdown/) for available options.

```json5
{
  /// ... other configurations ...
   "markdownJa": {
      "textWrap": "maintain",
      "emphasisKind": "underscores",
      "strongKind": "asterisks"
   },
  "plugins": [
    "/absolute/path/to/dprint_plugin_markdown_ja.wasm"
  ]
  /// ... other configurations ...
}
```

## License

MIT as the original plugin. See [LICENSE](./LICENSE) for more information.

## Acknowledgements

[dprint](https://dprint.dev/) and [dprint/dprint-plugin-markdown](https://github.com/dprint/dprint-plugin-markdown), obviously.
