# Contributing to soffit

## Prerequisites

- Rust toolchain (`rustup` recommended)
- Linux (build from source): GTK and WebKit dev headers

```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
```

## Build

```bash
cargo build
```

## Test

```bash
cargo test
```

## Run the editor locally

```bash
cargo run -- edit
```

## Test a widget

```bash
echo '{"session_id":"test","version":"1.0.0","model":{"display_name":"claude-sonnet-4-6"},"context_window":{"used_percentage":50.0}}' | cargo run -- render
```

## Widget development

See the [Custom Widgets](README.md#custom-widgets) section in the README for the widget format, input/output contract, and sidecar metadata.

## Pull requests

- Tests pass: `cargo test`
- No clippy warnings: `cargo clippy -- -W clippy::all`
- Formatted: `cargo fmt`

Keep changes focused. One feature or fix per PR.
