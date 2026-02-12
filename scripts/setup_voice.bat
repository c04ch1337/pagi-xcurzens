@echo off
REM setup_voice.bat - One-click Sovereign Voice setup (STT/TTS + optional local Whisper)
REM Creates ./models, optionally downloads ggml-base.en.bin, appends to .env.

setlocal
set ROOT=%~dp0
cd /d "%ROOT%"

echo.
echo [PAGI] Sovereign Voice Setup
echo ===========================
echo.

REM 1. Check libclang (needed for building with --features voice,whisper)
where clang >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [INFO] clang not in PATH. For local Whisper, install LLVM/clang and set LIBCLANG_PATH.
    echo        Without it, use STT_API_KEY for cloud transcription.
) else (
    echo [OK] clang found in PATH.
)

REM 2. Create models directory
if not exist "models" mkdir models
echo [OK] models directory ready.

REM 3. Check .env
if not exist ".env" (
    if exist ".env.example" (
        copy .env.example .env
        echo [OK] Created .env from .env.example
    ) else (
        echo [WARN] No .env.example found. Create .env manually.
    )
) else (
    echo [OK] .env exists.
)

REM 4. WHISPER_MODEL_PATH - if not set, offer to download ggml-base.en.bin
set "WHISPER_ENV="
for /f "usebackq tokens=*" %%a in (".env") do (
    echo %%a | findstr /b "WHISPER_MODEL_PATH=" >nul 2>&1
    if not errorlevel 1 set WHISPER_ENV=1
)
if not defined WHISPER_ENV (
    set "MODEL_PATH=%CD%\models\ggml-base.en.bin"
    if not exist "models\ggml-base.en.bin" (
        echo.
        echo [WHISPER] ggml-base.en.bin not found. Download from Hugging Face:
        echo   https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
        echo   Save to: %MODEL_PATH%
        echo   Or run: curl -L -o "models\ggml-base.en.bin" "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"
        echo.
        powershell -NoProfile -Command "& { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin' -OutFile 'models\ggml-base.en.bin' -UseBasicParsing }" 2>nul
        if exist "models\ggml-base.en.bin" (
            echo [OK] Downloaded ggml-base.en.bin to models\
        ) else (
            echo [SKIP] Download failed or skipped. Add WHISPER_MODEL_PATH to .env when you have the model.
        )
    )
    echo.>> .env
    echo # Voice: local Whisper model (set by setup_voice.bat)>> .env
    echo WHISPER_MODEL_PATH=%MODEL_PATH%>> .env
    echo [OK] Appended WHISPER_MODEL_PATH to .env
) else (
    echo [OK] WHISPER_MODEL_PATH already set in .env
)

REM 5. Remind about API keys for TTS/STT
echo.
echo [NEXT] For production voice:
echo   - TTS: set TTS_API_KEY or PAGI_LLM_API_KEY in .env
echo   - STT: set STT_API_KEY or use WHISPER_MODEL_PATH with: cargo build -p pagi-gateway --features voice
echo   - Run with voice: cargo run -p pagi-gateway --features voice -- --voice
echo.
endlocal
