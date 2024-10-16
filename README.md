# dprint-plugin-markdown-ja

Opinionated and hacky Markdown formatting plugin for [dprint](https://dprint.dev/), based on the [dprint/dprint-plugin-markdown](https://github.com/dprint/dprint-plugin-markdown) [v0.17.8](https://github.com/dprint/dprint-plugin-markdown/releases/tag/0.17.8), to support Japanese text.

## Features

The plugin will insert a space between Japanese and Latin letters with the following heuristically determined rules:

1. If the combination of previous and current characters is as follows, insert no space in-between:
   - both characters are Japanese
   - the previous character is Japanese and the current character is one of '(', ')', ':', '/', '_', '`', '*', '~'
   - the current character is Japanese and the previous character is one of '(', ')', '/', '_', '`', '*', '~'
   - the previous character is a Japanese symbol and punctuation character
   - the current character is a Japanese symbol and punctuation character
2. If the previous character is a space and the current character is not Japanese, insert a space.
3. If the previous character is not Japanese and the current character is a space, insert a space
4. If one character is Japanese and the other is not, insert a space

Please note that inserting a space between Japanese and Latin letters is a controversial topic. See [Do not insert whitespaces between latin and cj letters · Issue #6385 · prettier/prettier](https://github.com/prettier/prettier/issues/6385) for the discussion in the context of Prettier.

## How it Works

The plugin will determine whether a character is **Japanese** or **Japanese symbols and punctuations** based on the Unicode code points. See the following sections. See [Text_Japanese.txt](tests/specs/Text/Text_Japanese.txt) for how the plugin will work.

### Japanese Characters

| Range                                        | Description                             | Reference                                     |
|----------------------------------------------|-----------------------------------------|-----------------------------------------------|
| `\u{3041}..=\u{3096}`, `\u{3099}..=\u{309F}` | Hiragana                                | https://www.unicode.org/charts/PDF/U3040.pdf  |
| `\u{1B100}..=\u{1B12F}`                      | Kana Extended-A                         | https://www.unicode.org/charts/PDF/U1B100.pdf |
| `\u{1AFF0}..=\u{1AFFF}`                      | Kana Extended-B                         | https://www.unicode.org/charts/PDF/U1AFF0.pdf |
| `\u{1B000}..=\u{1B0FF}`                      | Kana Supplement                         | https://www.unicode.org/charts/PDF/U1B000.pdf |
| `\u{1B130}..=\u{1B16F}`                      | Small Kana Extension                    | https://www.unicode.org/charts/PDF/U1B130.pdf |
| `\u{30A0}..=\u{30FF}`                        | Katakana                                | https://www.unicode.org/charts/PDF/U30A0.pdf  |
| `\u{4E00}..=\u{9FFF}`                        | CJK Unified Ideographs                  | https://www.unicode.org/charts/PDF/U4E00.pdf  |
| `\u{3400}..=\u{4DBF}`                        | CJK Unified Ideographs Extension A      | https://www.unicode.org/charts/PDF/U3400.pdf  |
| `\u{20000}..=\u{2A6DF}`                      | CJK Unified Ideographs Extension B      | https://www.unicode.org/charts/PDF/U20000.pdf |
| `\u{2A700}..=\u{2B739}`                      | CJK Unified Ideographs Extension C      | https://www.unicode.org/charts/PDF/U2A700.pdf |
| `\u{2B740}..=\u{2B81D}`                      | CJK Unified Ideographs Extension D      | https://www.unicode.org/charts/PDF/U2B740.pdf |
| `\u{2B820}..=\u{2CEA1}`                      | CJK Unified Ideographs Extension E      | https://www.unicode.org/charts/PDF/U2B820.pdf |
| `\u{2CEB0}..=\u{2EBE0}`                      | CJK Unified Ideographs Extension F      | https://www.unicode.org/charts/PDF/U2CEB0.pdf |
| `\u{30000}..=\u{3134A}`                      | CJK Unified Ideographs Extension G      | https://www.unicode.org/charts/PDF/U30000.pdf |
| `\u{31350}..=\u{323AF}`                      | CJK Unified Ideographs Extension H      | https://www.unicode.org/charts/PDF/U31350.pdf |
| `\u{2EBF0}..=\u{2EE5D}`                      | CJK Unified Ideographs Extension H      | https://www.unicode.org/charts/PDF/U2EBF0.pdf |
| `\u{F900}..=\u{FAFF}`                        | CJK Compatibility Ideographs            | https://www.unicode.org/charts/PDF/UF900.pdf  |
| `\u{2F800}..=\u{2FA1F}`                      | CJK Compatibility Ideographs Supplement | https://www.unicode.org/charts/PDF/U2F800.pdf |
| Symbols and punctuations (see table below)   |                                         |                                               |

### Symbols and Punctuations

| Range                 | Description                 | Reference                                    |
|-----------------------|-----------------------------|----------------------------------------------|
| `\u{3000}..=\u{303F}` | CJK Symbols and Punctuation | https://www.unicode.org/charts/PDF/U3000.pdf |
| `\u{FF01}..=\u{FF60}` | Fullwidth Forms             | https://www.unicode.org/charts/PDF/UFF00.pdf |
| `\u{30fb}`            | Katakana Middle Dot         | https://www.unicode.org/charts/PDF/U30A0.pdf |

## How to Build a WASM file for `dprint`

To build a `.wasm` file, you have to install `wasm32-unknown-unknown` target for Rust. This can be done using the following command: 

```console
$ rustup target add wasm32-unknown-unknown
```

Then, you can build the `.wasm` file using the following command:

```console
$ cargo build --release --target=wasm32-unknown-unknown --features="wasm"
```

## How to Install

After the build, the `.wasm` file will be located at `target/wasm32-unknown-unknown/release/dprint_plugin_markdown_ja.wasm`. You can copy this file to the directory of yor choice and use it with `dprint`.

```json5
{
  /// ... other configurations ...
  "plugins": [
    "/absolute/path/to/dprint_plugin_markdown_ja.wasm"
  ]
  /// ... other configurations ...
}
```

## Configuration

See [Markdown Plugin](https://dprint.dev/plugins/markdown/). There are no additional configuration options specific to this plugin.

## License

MIT as the original plugin. See [LICENSE](./LICENSE) for more information.

## Acknowledgements

[dprint](https://dprint.dev/) and [dprint/dprint-plugin-markdown](https://github.com/dprint/dprint-plugin-markdown), obviously.
