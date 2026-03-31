---
name: setup-soffit
description: Install and configure soffit statusline for Claude Code
---

# Setup Soffit

Install soffit and configure your Claude Code statusline.

## Steps

1. Check if soffit is already installed: `which soffit`
2. If not installed:
   - Check if cargo is available: `which cargo`
   - If cargo available: `cargo install soffit`
   - If not: `curl -fsSL https://raw.githubusercontent.com/noxcraftdev/soffit/main/install.sh | sh`
3. Verify installation: `soffit widgets`
4. Read the current `~/.claude/settings.json`
5. Update the `statusLine` section to:
   ```json
   "statusLine": {
     "type": "command",
     "command": "soffit render",
     "padding": 0
   }
   ```
6. Create the config directory if needed: `mkdir -p ~/.config/soffit`
7. If no config exists, create a default: `soffit render` (this triggers config creation on first run)
8. Report success and suggest: `soffit edit` to customize, `soffit widgets` to see available widgets
