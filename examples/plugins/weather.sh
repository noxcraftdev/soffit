#!/bin/bash
# Example soffit plugin: weather widget (mock data)
#
# In a real plugin you would call a weather API and cache results to a temp
# file (e.g. /tmp/soffit-weather-cache) so the widget stays within the
# ~200ms render budget. Something like:
#
#   curl -s "https://wttr.in/?format=j1" > /tmp/soffit-weather-cache &
#   TEMP=$(jq -r '.current_condition[0].temp_C' /tmp/soffit-weather-cache 2>/dev/null)
#
# This example uses hardcoded values so it works out of the box.

INPUT=$(cat)

COMPACT=$(echo "$INPUT" | python3 -c "
import json, sys
d = json.load(sys.stdin)
print(d.get('config', {}).get('compact', False))
" 2>/dev/null)

COMPS=$(echo "$INPUT" | python3 -c "
import json, sys
d = json.load(sys.stdin)
print(','.join(d.get('config', {}).get('components', [])))
" 2>/dev/null)

# --- Mock values (replace with real API calls + caching) ---
TEMP="22°C"
COND="sunny"
HUMID="45%"
# -----------------------------------------------------------

parts=""
show_all=true
[ -n "$COMPS" ] && show_all=false

# Temperature component
if $show_all || echo "$COMPS" | grep -q "temp"; then
  [ "$COMPACT" = "True" ] && parts="$TEMP" || parts="🌡$TEMP"
fi

# Condition component
if $show_all || echo "$COMPS" | grep -q "condition"; then
  [ -n "$parts" ] && parts="$parts "
  [ "$COMPACT" = "True" ] && parts="${parts}${COND}" || parts="${parts}☀ ${COND}"
fi

# Humidity component
if $show_all || echo "$COMPS" | grep -q "humidity"; then
  [ -n "$parts" ] && parts="$parts "
  [ "$COMPACT" = "True" ] && parts="${parts}${HUMID}" || parts="${parts}💧${HUMID}"
fi

echo "{\"output\": \"$parts\", \"components\": [\"temp\", \"condition\", \"humidity\"]}"
