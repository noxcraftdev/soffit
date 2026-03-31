# soffit

[![Crates.io](https://img.shields.io/crates/v/soffit)](https://crates.io/crates/soffit)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/noxcraftdev/soffit/actions/workflows/release.yml/badge.svg)](https://github.com/noxcraftdev/soffit/actions)

Customizable statusline manager for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Desktop editor with drag-and-drop, live preview, and a plugin system for custom widgets.

![soffit statusline](assets/statusline.png)

## Features

- **9 built-in widgets**: context bar, cost, git, version, duration, vim mode, agent, quota, session
- **Desktop editor**: drag-and-drop widget ordering, live preview, per-widget component configuration
- **Plugin system**: create custom widgets as shell scripts or compiled binaries
- **Auto-detection**: plugins declare components via JSON output for full editor integration
- **Terminal-width aware**: automatic wrapping and responsive bar widths

## Install

### Pre-built binary (recommended)
```bash
curl -fsSL https://raw.githubusercontent.com/noxcraftdev/soffit/main/install.sh | sh
```

### From source
```bash
cargo install soffit
```

### System dependencies (Linux, build from source only)
```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
```

### Supported platforms

- Linux (x86_64)
- macOS (Intel and Apple Silicon)

## Setup

Add to `~/.claude/settings.json`:
```json
{
  "statusLine": {
    "type": "command",
    "command": "soffit render",
    "padding": 0
  }
}
```

## Usage

```bash
soffit render          # Render statusline (reads Claude Code JSON from stdin)
soffit edit            # Open the desktop config editor
soffit widgets         # List available widgets (built-in + plugins)
soffit widget <name>   # Test a single widget
```

## Configuration

Config lives at `~/.config/soffit/config.toml` (falls back to `~/.config/claude-statusline/config.toml`):

```toml
statusline_line1 = ["vim", "agent", "version", "context_bar", "quota", "duration", "cost"]
statusline_line2 = ["git", "insights"]
statusline_line3 = []

cost_target_weekly = 300.0
autocompact_pct = 100

[statusline_widgets.cost]
compact = false
components = ["session", "today", "week"]
```

## Custom Plugins

Drop scripts in `~/.config/soffit/plugins/`:

```bash
#!/bin/bash
# ~/.config/soffit/plugins/weather.sh
INPUT=$(cat)
COMPACT=$(echo "$INPUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('config',{}).get('compact',False))" 2>/dev/null)

TEMP="22°C"
COND="sunny"

if [ "$COMPACT" = "True" ]; then
  echo "{\"output\": \"$TEMP\", \"components\": [\"temp\", \"condition\"]}"
else
  echo "{\"output\": \"☀ $TEMP $COND\", \"components\": [\"temp\", \"condition\"]}"
fi
```

Make it executable: `chmod +x ~/.config/soffit/plugins/weather.sh`

### Plugin input format

Plugins receive JSON on stdin:
```json
{
  "data": {
    "session_id": "abc123",
    "version": "1.2.16",
    "model": {"display_name": "claude-sonnet-4-6"},
    "context_window": {"used_percentage": 42.0},
    "cost": {"total_duration_ms": 4830000, "total_cost_usd": 0.42},
    "vim": {"mode": "NORMAL"},
    "agent": {"name": "worker-1"}
  },
  "config": {
    "compact": false,
    "components": ["temp", "condition"]
  }
}
```

### Plugin output format

Return JSON to declare components (auto-detected in the editor):
```json
{"output": "rendered text here", "components": ["comp1", "comp2"]}
```

Or return plain text:
```
rendered text here
```

### Plugin metadata (optional)

Create a `.toml` sidecar for richer editor integration:
```toml
# ~/.config/soffit/plugins/weather.toml
description = "Current weather conditions"
components = ["temp", "condition"]
has_compact = true
```

## Community Plugins

Install plugins shared on GitHub without cloning manually:

```bash
# Install all plugins from a repo
soffit install user/repo

# Install a specific plugin from a repo
soffit install user/repo/plugin-name

# Remove an installed plugin
soffit uninstall plugin-name

# Overwrite an existing plugin
soffit install user/repo --force
```

Installed plugins land in `~/.config/soffit/plugins/` and are immediately available.

### Creating a plugin repository

Lay out your repo as a flat directory of `{name}.sh` + `{name}.toml` pairs:

```
my-soffit-plugins/
  weather.sh
  weather.toml
  stocks.sh
  stocks.toml
```

soffit looks for this layout at the repo root first, then inside a `plugins/` subdirectory. Multiple plugins per repo is the norm — a single repo can host an entire collection.

The `.toml` sidecar is optional but recommended: it supplies the description and component list shown in `soffit edit`.

## Editor

`soffit edit` opens a desktop GUI:

- **Lines tab**: drag-and-drop widgets across 3 statusline rows
- **Widgets tab**: configure built-in widgets (reorder components, toggle compact mode)
- **Plugin management**: create, edit, preview, rename, delete plugins
- **Live preview**: see your statusline update in real-time

![Editor - Lines tab](assets/editor-lines.png)
![Editor - Widgets tab](assets/editor-widgets.png)

<video src="assets/editor-demo.webm" autoplay loop muted playsinline width="100%"></video>

## License

MIT
