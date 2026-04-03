# soffit

[![Crates.io](https://img.shields.io/crates/v/soffit)](https://crates.io/crates/soffit)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/noxcraftdev/soffit/actions/workflows/release.yml/badge.svg)](https://github.com/noxcraftdev/soffit/actions)

Customizable statusline manager for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Desktop editor with drag-and-drop, live preview, and a plugin system for custom widgets.

![soffit in action](assets/soffit-live.png)

![soffit statusline](assets/statusline.png)

## Features

- **9 built-in widgets**: context bar, cost, git, version, duration, vim mode, agent, quota, session
- **Configurable theme**: custom colors, icons, and bar styles via config or the desktop editor
- **Desktop editor**: drag-and-drop widget ordering, live preview, per-widget component configuration
- **Plugin system**: create custom widgets as shell scripts or compiled binaries
- **Auto-detection**: plugins declare components via JSON output for full editor integration
- **Terminal-width aware**: automatic wrapping and responsive bar widths

## Install

### Pre-built binary (recommended)
```bash
curl -fsSL https://raw.githubusercontent.com/noxcraftdev/soffit/main/install.sh | sh
```

### Homebrew (macOS/Linux)
```bash
brew tap noxcraftdev/soffit
brew install soffit
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

### Theme colors

Override any color using ANSI 256-color indices:

```toml
[statusline_theme]
green = 82        # brighter green
red = 196         # pure red
dim = 240         # lighter gray
purple = 141      # different purple
```

Available color roles:
`green`, `orange`, `red`, `dim`, `lgray`, `cyan`, `purple`, `yellow`,
`dim_green`, `dim_yellow`, `dim_orange`, `dim_red`, `dim_cyan`, `dim_pink`.
Unset roles use the built-in defaults.

### Icons

Replace per-widget icons with any character or string:

```toml
[statusline_icons]
cost = "$ "          # instead of 💸
duration = "T "      # instead of ⏱
git_branch = " "    # nerd font branch icon
agent = "> "         # ASCII fallback
```

Available icon keys:
`duration`, `cost`, `git_branch`, `git_staged`, `agent`, `update`.

Bar characters can also be overridden:
`bar_fill`, `bar_empty`, `bar_half` (context bar),
`quota_fill`, `quota_empty`, `quota_pace` (quota bar).

### Bar style

Choose a preset for the quota progress bar:

```toml
bar_style = "block"   # ◎◉● density (default)
bar_style = "dot"     # ●○
bar_style = "ascii"   # #-
```

### Unicode text

Superscript/subscript rendering in the version widget can be toggled:

```toml
use_unicode_text = false   # plain text instead of ¹·²·³ / ₛₒₙₙₑₜ
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

Return JSON with `parts` so the framework reorders components per user config:
```json
{"parts": {"temp": "22°C", "condition": "sunny"}, "components": ["temp", "condition"]}
```

Or return a pre-composed string (component reordering won't apply):
```json
{"output": "22°C sunny", "components": ["temp", "condition"]}
```

Or return plain text:
```
22°C sunny
```

### Plugin metadata (optional)

Create a `.toml` sidecar for richer editor integration:
```toml
# ~/.config/soffit/plugins/weather.toml
description = "Current weather conditions"
components = ["temp", "condition"]
has_compact = true
```

## Plugin Marketplace

The marketplace subcommand manages a list of named plugin sources (GitHub repos that publish a `registry.json`).
By default soffit ships with the official `noxcraftdev/soffit-marketplace` source.

```bash
# Add a community source
soffit marketplace add community alice/soffit-extras

# List registered sources (no network)
soffit marketplace list

# List sources with plugin counts (fetches or uses cached registry)
soffit marketplace list --verbose

# Remove a source
soffit marketplace remove community

# Refresh the cached registry for all sources (or one with --source)
soffit marketplace update
soffit marketplace update --source community
```

### Installing from the marketplace

Once sources are configured, install by plugin name — soffit searches all sources:

```bash
soffit install <name>          # resolves from marketplace sources
soffit install owner/repo      # installs all plugins from a repo directly
soffit install owner/repo/name # installs a single plugin from a specific repo
```

### Publishing a marketplace source

Create a `registry.json` at the root of any public GitHub repo:

```json
{
  "plugins": [
    {
      "name": "weather",
      "description": "Current weather conditions",
      "repo": "alice/soffit-extras",
      "file": "weather.sh"
    }
  ]
}
```

Then share the source with: `soffit marketplace add your-source alice/soffit-extras`.

## Community Plugins

Install plugins shared on GitHub:

```bash
# Install all plugins from the official collection
soffit install noxcraftdev/soffit-plugins

# Install a specific plugin
soffit install noxcraftdev/soffit-plugins/last-msg

# Remove an installed plugin
soffit uninstall last-msg

# Overwrite an existing plugin
soffit install noxcraftdev/soffit-plugins --force
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
