# nile - String Validator

This repository contains OpenTTD's translation string validator for `nile`.

This tool validates if a translation is valid for a given base-string, by following all the (sometimes complex) rules for OpenTTD.

## Installation

Have Rust [installed](https://www.rust-lang.org/tools/install).

## Development

For easy local development:

* Validate base string:
    ```bash
    cargo run -- <base>
    ```
* Validate translation string:
    ```bash
    cargo run -- <base> <translation>
    ```

It will output the normalized string form, and whether the string is valid; and if not, what was wrong with it.

## WASM integration

This tool also integrates with WASM, so validation can be done from any website.
For this [wasm-pack](https://rustwasm.github.io/wasm-pack/) is used.

```bash
wasm-pack build --release
```

## API usage

### Step 1: Validate and normalize the base string

**API method:**
```rust
fn validate_base(config: LanguageConfig, base: String) -> ValidationResult
```

**Input:**
* `config.dialect`: One of `openttd`, `newgrf`, `game-script`.
* `config.cases`: Empty for base language.
* `config.genders`: Empty for base language.
* `config.plural_count`: `2` for base language.
* `base`: Base string to validate

**Output:**
* `errors`: List of errors. If this is not empty, the string should not be offered to translators.
* `normalized`: The normalized text to display to translators.
    * In the normalized text, string commands like `RAW_STRING`, `STRING5`, ... are replaced with `STRING`.
    * Translators can copy the normalized text as template for their translation.

**Example:**
```console
>>> cargo run "{BLACK}Age: {LTBLUE}{STRING2}{BLACK}   Running Cost: {LTBLUE}{CURRENCY}/year"
ERROR at position 61 to 71: Unknown string command '{CURRENCY}'.

>>> cargo run "{BLACK}Age: {LTBLUE}{STRING2}{BLACK}   Running Cost: {LTBLUE}{CURRENCY_LONG}/year"
NORMALIZED:{BLACK}Age: {LTBLUE}{0:STRING}{BLACK}   Running Cost: {LTBLUE}{1:CURRENCY_LONG}/year
```

### Step 2: Translators translates strings

* Translators must provide a text for the default case.
* Other cases are optional.
* Game-scripts do not support cases. There is a method in `LanguageConfig` to test for this, but it is not exported yet.

### Step 3: Validate and normalize the translation string

**API method:**
```rust
fn validate_translation(config: LanguageConfig, base: String, case: String, translation: String) -> ValidationResult
```

**Input:**
* `config.dialect`: One of `openttd`, `newgrf`, `game-script`.
* `config.cases`: `case` from `nile-config`.
* `config.genders`: `gender` from `nile-config`.
* `config.plural_count`: Number of plural forms from `nile-config`.
* `base`: Base string the translation is for.
* `case`: Case for the translation. Use `"default"` for the default case.
* `translation`: The text entered by the translator.

**Output:**
* `errors`: List of errors.
    * `severity`: Severity of the error.
        * `error`: The translation is broken, and must not be committed to OpenTTD.
        * `warning`: The translation is okay to commit, but translators should fix it anyway. This is used for new validations, which Eints did not do. So there are potentially lots of existing translations in violation.
    * `position`: Byte position in input string. `None`, if general message without location.
    * `message`: Error message.
    * `suggestion`: Some extended message with hints.
* `normalized`: The normalized text to committed. In the normalized text, trailing whitespace and other junk has been removed.

**Example:**
```console
>>> cargo run "{BLACK}Age: {LTBLUE}{STRING2}{BLACK}   Running Cost: {LTBLUE}{CURRENCY_LONG}/year" "{BLUE}Alter: {LTBLUE}{STRING}{BLACK} Betriebskosten: {LTBLUE}{0:CURRENCY_LONG}/Jahr"
ERROR at position 61 to 78: Duplicate parameter '{0:CURRENCY_LONG}'.
ERROR at position 61 to 78: Expected '{0:STRING2}', found '{CURRENCY_LONG}'.
ERROR: String command '{1:CURRENCY_LONG}' is missing.
WARNING: String command '{BLUE}' is unexpected. HINT: Remove this command.

>>> cargo run "{BLACK}Age: {LTBLUE}{STRING2}{BLACK}   Running Cost: {LTBLUE}{CURRENCY_LONG}/year" "{BLACK}Alter: {LTBLUE}{STRING}{BLACK} Betriebskosten: {LTBLUE}{CURRENCY_LONG}/Jahr"
NORMALIZED:{BLACK}Alter: {LTBLUE}{0:STRING}{BLACK} Betriebskosten: {LTBLUE}{1:CURRENCY_LONG}/Jahr
```
