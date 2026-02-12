#!/usr/bin/env bash
# Forge Hot-Reload Orchestrator - Enables dynamic skill activation without Gateway restart
#
# This script orchestrates the hot-reload process for Forge-generated skills:
# 1. Creates a new skill via the Forge API
# 2. Triggers incremental compilation
# 3. Activates the skill without full Gateway restart
#
# Usage:
#   ./forge-hot-reload.sh <skill_name> [description] [params_json]
#
# Examples:
#   ./forge-hot-reload.sh salesforce_sentinel "Scans Salesforce for security issues"
#   ./forge-hot-reload.sh weather_sentinel "Fetches weather data" '[{"name":"location","type":"string","required":true}]'

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Functions
forge_log() {
    echo -e "${CYAN}🔥 Forge: $1${NC}"
}

forge_success() {
    echo -e "${GREEN}🔥 Forge: $1${NC}"
}

forge_error() {
    echo -e "${RED}🔥 Forge: $1${NC}"
}

forge_warning() {
    echo -e "${YELLOW}🔥 Forge: $1${NC}"
}

# Parse arguments
SKILL_NAME="${1:-}"
DESCRIPTION="${2:-}"
PARAMS="${3:-[]}"
GATEWAY_URL="${GATEWAY_URL:-http://localhost:8000}"

if [ -z "$SKILL_NAME" ]; then
    forge_error "Usage: $0 <skill_name> [description] [params_json]"
    exit 1
fi

# Check if Gateway is running
forge_log "Checking Gateway status..."
if ! STATUS_RESPONSE=$(curl -s "$GATEWAY_URL/v1/status" 2>/dev/null); then
    forge_error "Gateway is not running. Start it with ./pagi-up.sh"
    exit 1
fi

PORT=$(echo "$STATUS_RESPONSE" | grep -o '"port":[0-9]*' | cut -d':' -f2)
forge_success "Gateway is running on port $PORT"

# Check hot-reload status
forge_log "Checking hot-reload status..."
HOT_RELOAD_STATUS=$(curl -s "$GATEWAY_URL/api/v1/forge/hot-reload/status")
ENABLED=$(echo "$HOT_RELOAD_STATUS" | grep -o '"enabled":[^,}]*' | cut -d':' -f2)

if [ "$ENABLED" = "true" ]; then
    forge_success "Hot-reload is enabled"
else
    forge_warning "Hot-reload is disabled. Skill will require manual Gateway restart."
    forge_warning "Enable with: curl -X POST $GATEWAY_URL/api/v1/forge/hot-reload/enable"
fi

# Create the skill
forge_log "Creating skill '$SKILL_NAME'..."

TOOL_SPEC=$(cat <<EOF
{
  "name": "$SKILL_NAME",
  "description": "$DESCRIPTION",
  "params": $PARAMS
}
EOF
)

CREATE_RESPONSE=$(curl -s -X POST "$GATEWAY_URL/api/v1/forge/create" \
    -H "Content-Type: application/json" \
    -d "$TOOL_SPEC")

SUCCESS=$(echo "$CREATE_RESPONSE" | grep -o '"success":[^,}]*' | cut -d':' -f2)
CARGO_CHECK_OK=$(echo "$CREATE_RESPONSE" | grep -o '"cargo_check_ok":[^,}]*' | cut -d':' -f2)

if [ "$SUCCESS" = "true" ] || [ "$CARGO_CHECK_OK" = "true" ]; then
    forge_success "Skill created successfully!"
    echo ""
    echo -e "${CYAN}📋 Skill Details:${NC}"
    
    SKILL_NAME_OUT=$(echo "$CREATE_RESPONSE" | grep -o '"skill_name":"[^"]*"' | cut -d'"' -f4)
    MODULE_NAME=$(echo "$CREATE_RESPONSE" | grep -o '"module_name":"[^"]*"' | cut -d'"' -f4)
    FILE_PATH=$(echo "$CREATE_RESPONSE" | grep -o '"file_path":"[^"]*"' | cut -d'"' -f4)
    HOT_RELOADED=$(echo "$CREATE_RESPONSE" | grep -o '"hot_reloaded":[^,}]*' | cut -d':' -f2)
    
    echo "  Name:        $SKILL_NAME_OUT"
    echo "  Module:      $MODULE_NAME"
    echo "  File:        $FILE_PATH"
    
    if [ "$HOT_RELOADED" = "true" ]; then
        COMPILE_TIME=$(echo "$CREATE_RESPONSE" | grep -o '"compilation_time_ms":[0-9]*' | cut -d':' -f2)
        echo -e "  Hot-Reload:  ${GREEN}✓ Activated${NC}"
        echo "  Compile Time: ${COMPILE_TIME}ms"
        echo ""
        forge_success "Skill is ready for immediate use!"
    else
        echo -e "  Hot-Reload:  ${YELLOW}✗ Not activated${NC}"
        echo ""
        forge_warning "Restart Gateway to activate: ./pagi-down.sh && ./pagi-up.sh"
    fi
else
    forge_error "Skill creation failed!"
    echo ""
    MESSAGE=$(echo "$CREATE_RESPONSE" | grep -o '"message":"[^"]*"' | cut -d'"' -f4)
    echo -e "${RED}Error: $MESSAGE${NC}"
    exit 1
fi

# List all hot-reloaded skills
echo ""
forge_log "Listing all hot-reloaded skills..."
LIST_RESPONSE=$(curl -s "$GATEWAY_URL/api/v1/forge/hot-reload/list")
COUNT=$(echo "$LIST_RESPONSE" | grep -o '"count":[0-9]*' | cut -d':' -f2)

if [ "$COUNT" -gt 0 ]; then
    echo ""
    echo -e "${CYAN}🔥 Hot-Reloaded Skills ($COUNT):${NC}"
    # Note: Parsing JSON in bash is limited. For production, use jq.
    echo "$LIST_RESPONSE" | grep -o '"skill_name":"[^"]*"' | cut -d'"' -f4 | while read -r skill; do
        echo "  • $skill"
    done
else
    echo "  No hot-reloaded skills yet."
fi

echo ""
forge_success "Forge operation complete!"
