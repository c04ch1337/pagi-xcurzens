#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════════════════════
# FORGE KILL SWITCH - Emergency Shutdown for Autonomous Evolution
# ══════════════════════════════════════════════════════════════════════════════
# This script immediately:
# 1. Sets PAGI_FORGE_SAFETY_ENABLED=true in .env
# 2. Kills all active cargo build processes
# 3. Logs the emergency shutdown to KB-08
#
# Usage:
#   chmod +x forge-kill-switch.sh
#   ./forge-kill-switch.sh
#
# Or create a desktop shortcut for one-click emergency stop.
# ══════════════════════════════════════════════════════════════════════════════

echo -e "\033[1;31m🚨 FORGE KILL SWITCH ACTIVATED\033[0m"
echo -e "\033[1;31m═══════════════════════════════════════════════════════════════\033[0m"

# Step 1: Update .env to re-enable safety
echo ""
echo -e "\033[1;33m[1/3] Re-enabling Forge Safety Gate...\033[0m"

ENV_PATH="./.env"
if [ -f "$ENV_PATH" ]; then
    if grep -q "PAGI_FORGE_SAFETY_ENABLED\s*=\s*false" "$ENV_PATH"; then
        # Replace false with true
        sed -i.bak 's/PAGI_FORGE_SAFETY_ENABLED\s*=\s*false/PAGI_FORGE_SAFETY_ENABLED=true/' "$ENV_PATH"
        echo -e "\033[1;32m✓ PAGI_FORGE_SAFETY_ENABLED set to true in .env\033[0m"
    elif grep -q "PAGI_FORGE_SAFETY_ENABLED\s*=\s*true" "$ENV_PATH"; then
        echo -e "\033[1;32m✓ PAGI_FORGE_SAFETY_ENABLED already set to true\033[0m"
    else
        # Add the setting if it doesn't exist
        echo "" >> "$ENV_PATH"
        echo "PAGI_FORGE_SAFETY_ENABLED=true" >> "$ENV_PATH"
        echo -e "\033[1;32m✓ PAGI_FORGE_SAFETY_ENABLED added to .env (set to true)\033[0m"
    fi
else
    echo -e "\033[1;33m⚠ .env file not found - creating with safety enabled\033[0m"
    echo "PAGI_FORGE_SAFETY_ENABLED=true" > "$ENV_PATH"
fi

# Step 2: Kill all cargo build processes
echo ""
echo -e "\033[1;33m[2/3] Terminating active cargo build processes...\033[0m"

CARGO_PIDS=$(pgrep -f "cargo")
if [ -n "$CARGO_PIDS" ]; then
    echo "$CARGO_PIDS" | while read -r pid; do
        kill -9 "$pid" 2>/dev/null
        echo -e "\033[1;32m✓ Killed cargo process (PID: $pid)\033[0m"
    done
else
    echo -e "\033[1;32m✓ No active cargo processes found\033[0m"
fi

# Also kill rustc processes (compilation in progress)
RUSTC_PIDS=$(pgrep -f "rustc")
if [ -n "$RUSTC_PIDS" ]; then
    echo "$RUSTC_PIDS" | while read -r pid; do
        kill -9 "$pid" 2>/dev/null
        echo -e "\033[1;32m✓ Killed rustc process (PID: $pid)\033[0m"
    done
fi

# Step 3: Log to KB-08 (if gateway is running)
echo ""
echo -e "\033[1;33m[3/3] Logging emergency shutdown...\033[0m"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%S.%3NZ")
LOG_ENTRY=$(cat <<EOF
{
  "event": "forge_kill_switch_activated",
  "timestamp": "$TIMESTAMP",
  "reason": "Emergency shutdown initiated by Coach Jamey",
  "action": "PAGI_FORGE_SAFETY_ENABLED set to true, all cargo/rustc processes terminated"
}
EOF
)

# Try to POST to the gateway's KB-08 logging endpoint (if it exists)
if curl -s -X POST "http://127.0.0.1:8000/api/v1/kb/soma/log" \
    -H "Content-Type: application/json" \
    -d "$LOG_ENTRY" \
    --max-time 2 > /dev/null 2>&1; then
    echo -e "\033[1;32m✓ Emergency shutdown logged to KB-08\033[0m"
else
    echo -e "\033[1;33m⚠ Could not log to KB-08 (gateway may be offline)\033[0m"
fi

# Final status
echo ""
echo -e "\033[1;32m═══════════════════════════════════════════════════════════════\033[0m"
echo -e "\033[1;32m✅ FORGE KILL SWITCH COMPLETE\033[0m"
echo ""
echo -e "\033[1;36mStatus:\033[0m"
echo -e "  \033[1;32m• Forge Safety Gate: ENABLED\033[0m"
echo -e "  \033[1;32m• Autonomous Evolution: DISABLED\033[0m"
echo -e "  \033[1;32m• Active Compilations: TERMINATED\033[0m"
echo ""
echo -e "\033[1;37mPhoenix will now require your approval for all code changes.\033[0m"
echo -e "\033[1;37mRestart the gateway to apply the new safety setting.\033[0m"
echo ""
