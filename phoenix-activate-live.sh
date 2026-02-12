#!/usr/bin/env bash
# Phoenix LIVE Mode Quick Activation
# Automatically configures .env for LIVE mode and restarts the gateway

set -e

echo -e "\033[1;36m🔥 PHOENIX LIVE MODE ACTIVATION 🔥\033[0m"
echo -e "\033[1;36m================================================\033[0m"
echo ""

# Step 1: Check if .env exists
if [ ! -f ".env" ]; then
    echo -e "\033[1;33m[1/4] Creating .env from .env.example...\033[0m"
    cp ".env.example" ".env"
    echo -e "\033[1;32m✅ .env file created\033[0m"
else
    echo -e "\033[1;32m[1/4] .env file already exists\033[0m"
fi

echo ""

# Step 2: Check current LLM mode
echo -e "\033[1;33m[2/4] Checking current LLM mode...\033[0m"
if grep -q "^PAGI_LLM_MODE=live" ".env"; then
    echo -e "\033[1;32m✅ Already set to LIVE mode\033[0m"
else
    current_mode=$(grep "^PAGI_LLM_MODE=" ".env" || echo "PAGI_LLM_MODE=mock")
    echo -e "\033[1;33m⚠️  Currently in MOCK mode: $current_mode\033[0m"
    echo -e "\033[1;33m   Updating to LIVE mode...\033[0m"
    
    # Update the file (macOS/BSD compatible)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' 's/^PAGI_LLM_MODE=mock/PAGI_LLM_MODE=live/' ".env"
    else
        sed -i 's/^PAGI_LLM_MODE=mock/PAGI_LLM_MODE=live/' ".env"
    fi
    
    echo -e "\033[1;32m✅ Updated to LIVE mode\033[0m"
fi

echo ""

# Step 3: Check API key
echo -e "\033[1;33m[3/4] Checking OpenRouter API key...\033[0m"
has_key=false

if grep -q "^PAGI_LLM_API_KEY=sk-or-v1-" ".env" || grep -q "^OPENROUTER_API_KEY=sk-or-v1-" ".env"; then
    echo -e "\033[1;32m✅ API key is configured\033[0m"
    has_key=true
else
    echo -e "\033[1;31m❌ No API key found\033[0m"
    echo ""
    echo -e "\033[1;33mYou need an OpenRouter API key to use LIVE mode.\033[0m"
    echo -e "\033[1;36mGet one at: https://openrouter.ai/keys\033[0m"
    echo ""
    
    read -p "Enter your OpenRouter API key (or press Enter to skip): " key
    
    if [[ $key == sk-or-v1-* ]]; then
        echo -e "\033[1;33m   Adding API key to .env...\033[0m"
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s|^PAGI_LLM_API_KEY=.*|PAGI_LLM_API_KEY=$key|" ".env"
            sed -i '' "s|^OPENROUTER_API_KEY=.*|OPENROUTER_API_KEY=$key|" ".env"
        else
            sed -i "s|^PAGI_LLM_API_KEY=.*|PAGI_LLM_API_KEY=$key|" ".env"
            sed -i "s|^OPENROUTER_API_KEY=.*|OPENROUTER_API_KEY=$key|" ".env"
        fi
        echo -e "\033[1;32m✅ API key added\033[0m"
        has_key=true
    elif [ -n "$key" ]; then
        echo -e "\033[1;33m⚠️  Invalid key format (should start with sk-or-v1-)\033[0m"
        echo -e "\033[0;37m   You can add it manually to .env later\033[0m"
    else
        echo -e "\033[1;33m⚠️  Skipped - you'll need to add it manually to .env\033[0m"
    fi
fi

echo ""

# Step 4: Restart gateway
echo -e "\033[1;33m[4/4] Restarting gateway...\033[0m"

if [ "$has_key" = false ]; then
    echo -e "\033[1;33m⚠️  Cannot start in LIVE mode without API key\033[0m"
    echo -e "\033[0;37m   Add your key to .env and run: ./phoenix-rise.sh\033[0m"
    exit 0
fi

# Kill existing gateway process
if pgrep -f "pagi-gateway" > /dev/null; then
    echo -e "\033[0;37m   Stopping existing gateway...\033[0m"
    pkill -f "pagi-gateway"
    sleep 2
fi

# Start new gateway in background
echo -e "\033[0;37m   Starting gateway in LIVE mode...\033[0m"
nohup cargo run -p pagi-gateway > gateway.log 2>&1 &

echo ""
echo -e "\033[1;36m================================================\033[0m"
echo -e "\033[1;32m✅ LIVE MODE ACTIVATED\033[0m"
echo ""
echo -e "\033[1;33mGateway is starting in the background...\033[0m"
echo -e "\033[1;33mWait 10-15 seconds for it to fully initialize.\033[0m"
echo ""
echo -e "\033[1;33mMonitor logs with:\033[0m"
echo -e "\033[0;37m  tail -f gateway.log\033[0m"
echo ""
echo -e "\033[1;36mThen test with:\033[0m"
echo -e "\033[0;37m  ./phoenix-live-sync.sh\033[0m"
echo ""
