#!/usr/bin/env bash
# Phoenix Log Redaction Script (Bash)
# Removes sensitive information from log files before sharing

set -e

LOG_FILE=""
REDACT_ALL=false

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
GRAY='\033[0;90m'
WHITE='\033[0;37m'
NC='\033[0m' # No Color

function redact_log_file() {
    local file_path="$1"
    
    if [ ! -f "$file_path" ]; then
        echo -e "${RED}❌ File not found: $file_path${NC}"
        return 1
    fi
    
    echo -e "${CYAN}🔒 Redacting: $file_path${NC}"
    
    local redacted_file="${file_path%.log}.redacted.log"
    
    # Perform redactions using sed
    sed -E \
        -e 's/sk-[a-zA-Z0-9_-]{20,}/sk-REDACTED/g' \
        -e 's/sk_[a-zA-Z0-9_-]{20,}/sk_REDACTED/g' \
        -e 's/Bearer [a-zA-Z0-9_-]+/Bearer REDACTED/g' \
        -e 's/"api_key":[[:space:]]*"[^"]+"/"api_key": "REDACTED"/g' \
        -e 's/api_key[[:space:]]*=[[:space:]]*"[^"]+"/api_key = "REDACTED"/g' \
        -e 's/openrouter_api_key[[:space:]]*=[[:space:]]*"[^"]+"/openrouter_api_key = "REDACTED"/g' \
        -e 's/password["[:space:]:=]+[^[:space:],}]+/password: REDACTED/g' \
        -e 's/"password":[[:space:]]*"[^"]+"/"password": "REDACTED"/g' \
        -e 's/\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/user@REDACTED.com/g' \
        -e 's/\b([0-9]{1,3}\.){3}[0-9]{1,3}\b/XXX.XXX.XXX.XXX/g' \
        -e 's|/home/[^/]+|/home/REDACTED|g' \
        -e 's|/Users/[^/]+|/Users/REDACTED|g' \
        -e 's|C:\\Users\\[^\\]+|C:\\Users\\REDACTED|g' \
        "$file_path" > "$redacted_file"
    
    # Restore localhost IPs
    sed -i.bak 's/XXX\.XXX\.XXX\.XXX:8080/127.0.0.1:8080/g' "$redacted_file"
    sed -i.bak 's/XXX\.XXX\.XXX\.XXX:6333/127.0.0.1:6333/g' "$redacted_file"
    rm -f "${redacted_file}.bak"
    
    echo -e "${GREEN}✓ Saved to: $redacted_file${NC}"
    return 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --all|-a)
            REDACT_ALL=true
            shift
            ;;
        --help|-h)
            echo -e "${YELLOW}Usage:${NC}"
            echo -e "  ${WHITE}./redact-logs.sh <path-to-log>${NC}"
            echo -e "  ${WHITE}./redact-logs.sh --all${NC}"
            echo ""
            echo -e "${YELLOW}Examples:${NC}"
            echo -e "  ${GRAY}./redact-logs.sh ~/.pagi/logs/gateway.log${NC}"
            echo -e "  ${GRAY}./redact-logs.sh --all${NC}"
            echo ""
            exit 0
            ;;
        *)
            LOG_FILE="$1"
            shift
            ;;
    esac
done

# Main execution
echo ""
echo -e "${MAGENTA}🔒 Phoenix Log Redaction Tool${NC}"
echo -e "${GRAY}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

if [ "$REDACT_ALL" = true ]; then
    # Redact all logs in ~/.pagi/logs/
    LOGS_DIR="$HOME/.pagi/logs"
    
    if [ ! -d "$LOGS_DIR" ]; then
        echo -e "${RED}❌ Logs directory not found: $LOGS_DIR${NC}"
        echo -e "${YELLOW}   Have you run Phoenix yet?${NC}"
        exit 1
    fi
    
    # Find all .log files
    mapfile -t log_files < <(find "$LOGS_DIR" -maxdepth 1 -name "*.log" -type f)
    
    if [ ${#log_files[@]} -eq 0 ]; then
        echo -e "${YELLOW}⚠️  No log files found in $LOGS_DIR${NC}"
        exit 0
    fi
    
    echo -e "${CYAN}Found ${#log_files[@]} log file(s)${NC}"
    echo ""
    
    success_count=0
    for log_file in "${log_files[@]}"; do
        if redact_log_file "$log_file"; then
            ((success_count++))
        fi
    done
    
    echo ""
    echo -e "${GRAY}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}✓ Redacted $success_count/${#log_files[@]} log files${NC}"
    echo ""
    echo -e "${CYAN}Redacted logs saved with .redacted.log extension${NC}"
    echo -e "${CYAN}You can now safely share these files for debugging${NC}"
    
elif [ -z "$LOG_FILE" ]; then
    # No file specified, show usage
    echo -e "${YELLOW}Usage:${NC}"
    echo -e "  ${WHITE}./redact-logs.sh <path-to-log>${NC}"
    echo -e "  ${WHITE}./redact-logs.sh --all${NC}"
    echo ""
    echo -e "${YELLOW}Examples:${NC}"
    echo -e "  ${GRAY}./redact-logs.sh ~/.pagi/logs/gateway.log${NC}"
    echo -e "  ${GRAY}./redact-logs.sh --all${NC}"
    echo ""
    
else
    # Redact single file
    if redact_log_file "$LOG_FILE"; then
        echo ""
        echo -e "${GRAY}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${GREEN}✓ Log redaction complete${NC}"
        echo ""
        echo -e "${CYAN}You can now safely share the .redacted.log file${NC}"
    fi
fi

echo ""
