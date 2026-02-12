# 🔥 First Mission: Operation First Rise (Beta Stress Test)

This mission is the end-to-end proof that Phoenix is running **locally**, that the **sidecars** are healthy, and that **Concise Mode** is truly **diagram-first** via the JSON Diagram Envelope.

---

## ✅ Target Setup (Assumed)

- **OS**: Windows 11
- **UI**: Phoenix Studio UI (the browser window Phoenix opens on launch)
- **What you will submit** (3 artifacts):
  1. DiagramViewer screenshot
  2. Raw gateway JSON envelope
  3. Sidecar health evidence (terminal log snippet)

---

## 1) Launch Phoenix (Binaries + Sidecars)

1. Start Phoenix:

   - PowerShell: run `phoenix-rise.ps1`
   - Git Bash / WSL: run `phoenix-rise.sh`

2. Wait until the terminal indicates the backend is listening and any configured sidecars (ex: Qdrant) have started.

**Capture #1 (Sidecar evidence):**

- Copy/paste (or screenshot) the terminal lines that show:
  - sidecar process started (PID is a bonus)
  - health check OK (or “ready”)
  - any port used (ex: Qdrant typically `6333`)

---

## 2) Verify the “Architect’s View” (Concise Mode must be diagram-first)

1. In the UI, open Settings.
2. Set `density_mode` to **Concise**.
3. Ask this exact question:

> How does the Sovereign Firewall handle an unauthorized external API call?

**Expected behavior:**

- Response begins with the **JSON Diagram Envelope**.
- Envelope contains Mermaid with the dark theme init flag:

```text
%%{init: {'theme': 'dark'}}%%
```

- Prose is limited to **exactly 1–2 bullet points** *below* the diagram.

**Capture #2 (DiagramViewer screenshot):**

- Screenshot showing:
  - the diagram rendered (not raw code)
  - dark theme rendering
  - the 1–2 bullet summary below the diagram

---

## 3) Capture the Raw JSON Envelope (Gateway Output Proof)

1. Open DevTools in the UI:
   - Chrome / Edge: `F12`
2. Go to **Network**.
3. Find the `POST` request for chat (often `/api/v1/chat`).
4. Copy the **Response** body JSON.

**Capture #3 (Raw JSON):**

- Paste the full response JSON into your submission.
- Confirm it contains:
  - a top-level envelope object (no Markdown wrapper)
  - a Mermaid payload field (diagram code)
  - the dark theme init directive

---

## 4) Multi-Mode Check (Verbose must allow narrative-first)

1. Switch `density_mode` to **Verbose**.
2. Ask the same question again.

**Expected behavior:**

- The response becomes narrative-first.
- Diagrams are optional/supplementary unless explicitly requested.

---

## ✅ Submission Format (Copy/Paste)

Include these items in one message or issue:

1. **DiagramViewer screenshot** (image)
2. **Raw JSON envelope** (code block)
3. **Sidecar health evidence** (log snippet)
4. **One-line confirmation**:

> “The Architect’s View is locked in. Concise mode is now a visual-first experience.”

---

## 🧾 Optional: Machine-Check Your Submission (Mission Validator)

Phoenix includes a submission validator endpoint that can audit your bundle automatically.

- Validator guide: [`MISSION_SUBMISSION_VALIDATOR.md`](MISSION_SUBMISSION_VALIDATOR.md)


