#!/usr/bin/env bash
# Phoenix LIVE Mode Force-Sync
# Forces Phoenix to bypass mock/generic responses and engage the Sovereign Stack

set -e

echo -e "\033[1;36m🔥 PHOENIX LIVE MODE FORCE-SYNC 🔥\033[0m"
echo -e "\033[1;36m================================================\033[0m"
echo ""

# Step 1: Connection Test
echo -e "\033[1;33m[1/5] Testing Gateway Connection...\033[0m"
if health=$(curl -s http://localhost:8000/api/v1/health); then
    identity=$(echo "$health" | grep -o '"identity":"[^"]*"' | cut -d'"' -f4)
    message=$(echo "$health" | grep -o '"message":"[^"]*"' | cut -d'"' -f4)
    echo -e "\033[1;32m✅ Gateway LIVE: $identity\033[0m"
    echo -e "\033[0;37m   Message: $message\033[0m"
else
    echo -e "\033[1;31m❌ FAILED: Gateway not responding on port 8000\033[0m"
    echo -e "\033[1;33m   Run: ./phoenix-rise.sh\033[0m"
    exit 1
fi

echo ""

# Step 2: Check Forge Safety Status
echo -e "\033[1;33m[2/5] Checking Forge Safety Governor...\033[0m"
if safety=$(curl -s http://localhost:8000/api/v1/forge/safety-status 2>/dev/null); then
    if [ -n "$safety" ] && [ "$safety" != "" ]; then
        mode=$(echo "$safety" | grep -o '"mode":"[^"]*"' | cut -d'"' -f4)
        echo -e "\033[1;32m✅ Forge Status: $mode\033[0m"
        if echo "$safety" | grep -q '"safety_enabled":true'; then
            echo -e "\033[1;36m   Safety: ENABLED (HITL Mode)\033[0m"
        else
            echo -e "\033[1;35m   Safety: DISABLED (Autonomous)\033[0m"
        fi
    else
        echo -e "\033[1;33m⚠️  Forge endpoint returned empty (skill may not be registered)\033[0m"
    fi
else
    echo -e "\033[1;33m⚠️  Forge Safety endpoint not available\033[0m"
fi

echo ""

# Step 3: Check KB-08 (Soma) for last interaction
echo -e "\033[1;33m[3/5] Accessing KB-08 (Soma) for context...\033[0m"
if kb_status=$(curl -s http://localhost:8000/api/v1/kb-status); then
    echo -e "\033[1;32m✅ Knowledge Base Status:\033[0m"
    echo "$kb_status" | grep -o '"kb[0-9]*":[0-9]*' | while read -r line; do
        kb_name=$(echo "$line" | cut -d'"' -f2)
        kb_count=$(echo "$line" | cut -d':' -f2)
        echo -e "\033[0;37m   $kb_name : $kb_count entries\033[0m"
    done
else
    echo -e "\033[1;33m⚠️  Could not retrieve KB status\033[0m"
fi

echo ""

# Step 4: Test Chat Endpoint with Sovereign Context
echo -e "\033[1;33m[4/5] Testing Chat Endpoint with Sovereign Context...\033[0m"

chat_payload='{
  "prompt": "Phoenix, confirm you are operating in LIVE mode with full access to the Sovereign Stack. What was our last conversation topic?",
  "user_alias": "Coach The Creator",
  "agent_id": "phoenix"
}'

if response=$(curl -s -X POST http://localhost:8000/api/v1/chat \
    -H "Content-Type: application/json" \
    -d "$chat_payload"); then
    
    echo -e "\033[1;32m✅ Chat Response Received:\033[0m"
    echo ""
    echo -e "\033[1;36m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
    echo "$response" | grep -o '"response":"[^"]*"' | cut -d'"' -f4 | sed 's/\\n/\n/g'
    echo -e "\033[1;36m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\033[0m"
    echo ""
    
    # Check if response contains generic/mock indicators
    if echo "$response" | grep -qi "Thank you for reaching out\|I'm here to help\|How can I assist\|I don't have access to\|I cannot access"; then
        echo -e "\033[1;33m⚠️  WARNING: Response contains generic/mock patterns\033[0m"
        echo -e "\033[1;33m   Phoenix is in MOCK MODE - not using real AI inference\033[0m"
        echo ""
        echo -e "\033[1;36m🔧 TO FIX:\033[0m"
        echo -e "\033[0;37m   1. Edit .env file: Set PAGI_LLM_MODE=live\033[0m"
        echo -e "\033[0;37m   2. Add your OpenRouter API key: PAGI_LLM_API_KEY=sk-or-v1-...\033[0m"
        echo -e "\033[0;37m   3. Restart gateway: ./phoenix-rise.sh\033[0m"
        echo ""
        echo -e "\033[0;37m   📖 Full guide: PHOENIX_LIVE_MODE_ACTIVATION.md\033[0m"
    else
        echo -e "\033[1;32m✅ Response appears to be from LIVE Sovereign Stack\033[0m"
    fi
else
    echo -e "\033[1;31m❌ Chat endpoint failed\033[0m"
fi

echo ""

# Step 5: Verify Skill Registry
echo -e "\033[1;33m[5/5] Checking Skill Registry...\033[0m"
if skills=$(curl -s http://localhost:8000/api/v1/skills); then
    echo -e "\033[1;32m✅ Registered Skills:\033[0m"
    echo "$skills" | grep -o '"name":"[^"]*"' | cut -d'"' -f4 | while read -r skill; do
        echo -e "\033[0;37m   • $skill\033[0m"
    done
else
    echo -e "\033[1;33m⚠️  Could not retrieve skill registry\033[0m"
fi

echo ""
echo -e "\033[1;36m================================================\033[0m"
echo -e "\033[1;36m🔥 LIVE MODE SYNC COMPLETE 🔥\033[0m"
echo ""
echo -e "\033[1;33mNext Steps:\033[0m"
echo -e "\033[0;37m1. If Phoenix is still using mock responses, check the frontend connection\033[0m"
echo -e "\033[0;37m2. Verify the UI is pointing to http://localhost:8000\033[0m"
echo -e "\033[0;37m3. Check browser console for connection errors\033[0m"
echo -e "\033[0;37m4. Try: ./forge-kill-switch.sh status\033[0m"
echo ""
