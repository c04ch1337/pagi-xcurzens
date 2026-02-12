async function runAutonomousGoal() {
  const intent = document.getElementById("intent")?.value ?? "";
  const tenantId = document.getElementById("tenant")?.value ?? "default";
  const out = document.getElementById("out");
  if (out) out.textContent = "Running...";

  const body = {
    tenant_id: tenantId,
    goal: {
      AutonomousGoal: {
        intent,
        context: null,
      },
    },
  };

  const res = await fetch("/v1/execute", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });

  const text = await res.text();
  if (out) out.textContent = text;
}

document.getElementById("run")?.addEventListener("click", () => {
  runAutonomousGoal().catch((e) => {
    const out = document.getElementById("out");
    if (out) out.textContent = String(e);
  });
});

// ========== Sovereignty Status Bar (Kill Switch + Auto-Revert UI) ==========

async function fetchForgeSafetyStatus() {
  try {
    const res = await fetch("/api/v1/forge/safety-status");
    const data = await res.json();
    updateSovereigntyUI(data);
  } catch (e) {
    console.error("Failed to fetch forge safety status:", e);
  }
}

function updateSovereigntyUI(data) {
  const indicator = document.getElementById("sovereignty-indicator");
  const label = document.getElementById("sovereignty-label");
  
  if (!indicator || !label) return;
  
  const safetyEnabled = data.safety_enabled ?? true;
  const mode = data.mode ?? "HITL";
  
  if (safetyEnabled) {
    indicator.className = "sovereignty-indicator hitl";
    label.textContent = "HITL";
    label.title = "Human-in-the-Loop Mode (Safety: ENABLED)";
  } else {
    indicator.className = "sovereignty-indicator autonomous";
    label.textContent = "AUTONOMOUS";
    label.title = "Autonomous Evolution Mode (Safety: DISABLED)";
  }
}

async function triggerKillSwitch() {
  const confirmed = confirm(
    "🛡️ FORGE KILL SWITCH\n\n" +
    "This will immediately re-enable the Safety Governor and lock the Forge.\n\n" +
    "Phoenix will revert to HITL (Human-in-the-Loop) mode.\n\n" +
    "Continue?"
  );
  
  if (!confirmed) return;
  
  try {
    const res = await fetch("/api/v1/forge/safety", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ enabled: true }),
    });
    
    const data = await res.json();
    
    if (data.status === "ok") {
      console.log("✅ Kill Switch activated:", data.message);
      updateSovereigntyUI({ safety_enabled: true, mode: "HITL" });
    } else {
      console.error("❌ Kill Switch failed:", data.message);
      alert("Kill Switch failed: " + data.message);
    }
  } catch (e) {
    console.error("Kill Switch error:", e);
    alert("Kill Switch error: " + e.message);
  }
}

// Wire up kill switch button
document.getElementById("kill-switch")?.addEventListener("click", triggerKillSwitch);

// Poll forge safety status every 2 seconds (to detect auto-revert)
fetchForgeSafetyStatus();
setInterval(fetchForgeSafetyStatus, 2000);

// Optional: Click indicator to toggle (for testing)
document.getElementById("sovereignty-indicator")?.addEventListener("click", async () => {
  try {
    const res = await fetch("/api/v1/forge/safety-status");
    const data = await res.json();
    const currentState = data.safety_enabled ?? true;
    
    const newState = !currentState;
    const mode = newState ? "HITL" : "Autonomous";
    
    const confirmed = confirm(
      `Switch to ${mode} mode?\n\n` +
      (newState
        ? "This will ENABLE the Safety Governor (HITL mode)."
        : "⚠️ This will DISABLE the Safety Governor (Autonomous Evolution mode).\n\nPhoenix will be able to modify her own source code without approval.")
    );
    
    if (!confirmed) return;
    
    const setRes = await fetch("/api/v1/forge/safety", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ enabled: newState }),
    });
    
    const setData = await setRes.json();
    if (setData.status === "ok") {
      updateSovereigntyUI({ safety_enabled: newState, mode: setData.mode });
    }
  } catch (e) {
    console.error("Toggle failed:", e);
  }
});

