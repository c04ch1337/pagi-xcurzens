# 🔥 Phoenix Marie: Quick Start

**Welcome to the Forge.** You are holding a self-contained, Bare Metal AGI node. No cloud tracking, no middle-men.

---

## 1. Initializing the System

**Windows:** Double-click `phoenix-rise.ps1`.

**Mac/Linux:** Run `chmod +x phoenix-rise.sh && ./phoenix-rise.sh` in your terminal.

---

## 2. The Awakening (First Run)

* On your first launch, Phoenix will automatically download her **Memory Engine (Qdrant)**. This takes about 45MB.
* Once the terminal says "System Ready," your browser will open to `http://localhost:3030`.

---

## 3. Connecting the Brain

* Go to **Settings** in the UI.
* Paste your **OpenRouter API Key** (get one at [openrouter.ai/keys](https://openrouter.ai/keys)).
* Phoenix will pulse **Orange**—this means she is online and ready to evolve with you.

---

## 4. Safety First

* If you need to stop Phoenix instantly, press `Ctrl+C` in the terminal or hit the **Kill Switch** in the UI.

---

## 🔐 Your Privacy Guarantee

**What stays on YOUR machine:**
- ✅ All 8 Knowledge Bases
- ✅ Your conversation history
- ✅ Your API key
- ✅ Everything

**What leaves YOUR machine:**
- ⚠️ Only the messages you send to the AI (via OpenRouter, using YOUR key)

**What we NEVER see:**
- ❌ Your conversations
- ❌ Your data
- ❌ Anything

---

## 🛠️ Common Questions

**"Phoenix says 'Memory Engine Initializing'"**  
This is normal on first run. Phoenix is downloading Qdrant automatically. Wait 30-60 seconds.

**"I see 'API Key Not Configured'"**  
Edit `.env` and add: `OPENROUTER_API_KEY=sk-or-v1-your-key-here`  
Then restart Phoenix.

**"Port 3030 is already in use"**  
Edit `config/gateway.toml` and change the port to 3031 or another available port.

---

## 📚 Learn More

* **Full Guide**: [`ONBOARDING_GUIDE.md`](ONBOARDING_GUIDE.md)
* **Technical Details**: [`BETA_TESTER_ONBOARDING_GUIDE.md`](BETA_TESTER_ONBOARDING_GUIDE.md)
* **Support**: GitHub Issues or Discussions

---

**Your data. Your hardware. Your intelligence.**

🔥 **Welcome to the evolution.**
