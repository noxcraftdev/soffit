#!/bin/bash
# Example soffit widget: Docker container status
#
# Shows the count of running containers and (in non-compact mode) the names
# of the first three. Gracefully degrades when Docker is not running.

INPUT=$(cat)

COMPACT=$(echo "$INPUT" | python3 -c "
import json, sys
d = json.load(sys.stdin)
print(d.get('config', {}).get('compact', False))
" 2>/dev/null)

# Bail fast when Docker daemon is unreachable — avoids a multi-second timeout
# hanging the statusline render.
if ! docker info &>/dev/null; then
  echo '{"output": "🐳 off", "components": ["count", "names"]}'
  exit 0
fi

COUNT=$(docker ps -q 2>/dev/null | wc -l | tr -d ' ')

if [ "$COMPACT" = "True" ]; then
  echo "{\"output\": \"🐳${COUNT}\", \"components\": [\"count\", \"names\"]}"
  exit 0
fi

if [ "$COUNT" = "0" ]; then
  echo '{"output": "🐳 no containers", "components": ["count", "names"]}'
  exit 0
fi

# Limit to 3 names so the widget doesn't swamp the statusline
NAMES=$(docker ps --format '{{.Names}}' 2>/dev/null | head -3 | paste -sd ',' -)
echo "{\"output\": \"🐳 ${COUNT} running: ${NAMES}\", \"components\": [\"count\", \"names\"]}"
