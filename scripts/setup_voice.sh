#!/usr/bin/env bash
# setup_voice.sh - One-click Sovereign Voice setup (STT/TTS + optional local Whisper)
# Creates ./models, optionally downloads ggml-base.en.bin, appends to .env.

set -e
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

echo ""
echo "[PAGI] Sovereign Voice Setup"
echo "============================"
echo ""

# 1. Check libclang (needed for building with --features voice,whisper)
if command -v clang >/dev/null 2>&1; then
  echo "[OK] clang found in PATH."
else
  echo "[INFO] clang not in PATH. For local Whisper, install LLVM/clang and set LIBCLANG_PATH."
  echo "       Without it, use STT_API_KEY for cloud transcription."
fi

# 2. Create models directory
mkdir -p models
echo "[OK] models directory ready."

# 3. Check .env
if [ ! -f .env ]; then
  if [ -f .env.example ]; then
    cp .env.example .env
    echo "[OK] Created .env from .env.example"
  else
    echo "[WARN] No .env.example found. Create .env manually."
  fi
else
  echo "[OK] .env exists."
fi

# 4. WHISPER_MODEL_PATH - if not set, offer to download ggml-base.en.bin
if ! grep -q '^WHISPER_MODEL_PATH=' .env 2>/dev/null; then
  MODEL_PATH="$ROOT/models/ggml-base.en.bin"
  if [ ! -f "models/ggml-base.en.bin" ]; then
    echo ""
    echo "[WHISPER] Downloading ggml-base.en.bin from Hugging Face..."
    if command -v curl >/dev/null 2>&1; then
      curl -sL -o "models/ggml-base.en.bin" \
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin" || true
    elif command -v wget >/dev/null 2>&1; then
      wget -q -O "models/ggml-base.en.bin" \
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin" || true
    else
      echo "[WHISPER] Install curl or wget, or download manually:"
      echo "  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"
      echo "  Save to: $MODEL_PATH"
    fi
    if [ -f "models/ggml-base.en.bin" ]; then
      echo "[OK] Downloaded ggml-base.en.bin to models/"
    else
      echo "[SKIP] Download failed or skipped. Add WHISPER_MODEL_PATH to .env when you have the model."
    fi
  fi
  echo "" >> .env
  echo "# Voice: local Whisper model (set by setup_voice.sh)" >> .env
  echo "WHISPER_MODEL_PATH=$MODEL_PATH" >> .env
  echo "[OK] Appended WHISPER_MODEL_PATH to .env"
else
  echo "[OK] WHISPER_MODEL_PATH already set in .env"
fi

# 5. Remind about API keys for TTS/STT
echo ""
echo "[NEXT] For production voice:"
echo "  - TTS: set TTS_API_KEY or PAGI_LLM_API_KEY in .env"
echo "  - STT: set STT_API_KEY or use WHISPER_MODEL_PATH with: cargo build -p pagi-gateway --features voice"
echo "  - Run with voice: cargo run -p pagi-gateway --features voice -- --voice"
echo ""
