# nile-library - Library supporting nile

This repository contains the libirary that supports OpenTTD's translation tool `nile`.

This library for example validates if a translation is valid for a given base-string, and converts base-strings into a translatable form.

## Installation

Have Rust [installed](https://www.rust-lang.org/tools/install).

## Development

For easy local development:

```bash
cargo run -- <base> <case> <translation>
```

It will output whether the translation is valid, and if not, what was wrong with it.

## WASM integration

This tool also integrates with WASM, so validation can be done from any website.
For this [wasm-pack](https://rustwasm.github.io/wasm-pack/) is used.

```bash
wasm-pack build --release
```
