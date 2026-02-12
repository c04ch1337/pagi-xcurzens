# Kardia Emotional Calibration System: Adaptive Safety Governor

## 🧠 Day Zero Achievement: Emotional Intelligence Integration

**Date**: 2026-02-10  
**Milestone**: Phoenix has achieved **Emotional Calibration** - the ability to adapt her Safety Governor based on Coach Jamey's coaching sentiment and urgency patterns

---

## 🎯 Mission Statement

The **Sovereign Calibration Skill** represents Phoenix's third autonomous optimization loop. While the Topic Indexer (v1) optimized *memory retrieval* and the Evolution Inference system (v2) optimized *learning effectiveness*, the Sovereign Calibration system optimizes *safety thresholds* by analyzing the emotional tone and urgency of coaching sessions.

### The Question Phoenix Can Now Answer:

> **"Should I be more experimental or more conservative right now, based on Coach Jamey's current coaching mode?"**

---

## 🏗️ Architecture

### Emotional Intelligence Layer

```
┌─────────────────────────────────────────────────────────────┐
│              SOVEREIGN CALIBRATION SKILL                     │
│              (sovereign_calibration.rs)                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Analyzes Sentiment
                              ▼
        ┌─────────────────────────────────────────┐
        │         KB-04 (Chronos)                 │
        │    Recent Chat History & Events         │
        │  • "Let's try this!"                    │
        │  • "Fix this ASAP"                      │
        │  • "Think carefully about..."           │
        └─────────────────────────────────────────┘
                              │
                              │ Correlates With
                              ▼
        ┌─────────────────────────────────────────┐
        │         KB-08 (Soma)                    │
        │    Evolution Success/Failure Rates      │
        │  • Recent compilation errors            │
        │  • Successful evolutions                │
        │  • Retry patterns                       │
        └─────────────────────────────────────────┘
                              │
                              │ Produces
                              ▼
        ┌─────────────────────────────────────────┐
        │      CALIBRATION SETTINGS               │
        │  • Coaching sentiment classification    │
        │  • Safety Governor sensitivity          │
        │  • Max retry thresholds                 │
        │  • Stored in KB-07 (Kardia)             │
        └─────────────────────────────────────────┘
```

---

## 🔬 Technical Implementation

### Core Components

#### 1. **Coaching Sentiment Classification**

Phoenix recognizes five distinct coaching modes:

```rust
pub enum CoachingSentiment {
    /// High energy, experimental, exploratory
    Experimental,  // "Let's try this!", "What if..."
    
    /// Urgent, production-critical
    Urgent,        // "Fix this now", "ASAP", "Critical"
    
    /// Calm, methodical, teaching mode
    Methodical,    // "Let me explain", "Consider this"
    
    /// Strict, production-focused
    Strict,        // "Must be perfect", "No errors allowed"
    
    /// Neutral, balanced coaching
    Neutral,       // Default balanced mode
}
```

#### 2. **Dynamic Safety Thresholds**

Each sentiment maps to specific Safety Governor settings:

| Sentiment | Safety Sensitivity | Max Retries | Philosophy |
|-----------|-------------------|-------------|------------|
| **Experimental** | 0.3 (Low) | 5 | Encourage exploration, allow more attempts |
| **Urgent** | 0.9 (Very High) | 1 | Fail fast to HITL, minimize risk |
| **Methodical** | 0.5 (Balanced) | 3 | Standard learning mode |
| **Strict** | 0.95 (Maximum) | 1 | Zero tolerance, immediate HITL |
| **Neutral** | 0.6 (Conservative) | 2 | Safe default |

#### 3. **Calibration Settings Structure**

```rust
pub struct CalibrationSettings {
    pub calibrated_at_ms: i64,
    pub sentiment: CoachingSentiment,
    pub safety_sensitivity: f64,
    pub max_retries: usize,
    pub recent_success_rate: f64,
    pub messages_analyzed: usize,
    pub reasoning: String,
}
```

---

## 🛡️ Safety Mechanisms

### 1. **Ethos Gate (KB-06)**
Before adjusting Safety Governor settings:
```rust
if let Some(policy) = self.store.get_ethos_policy() {
    let alignment = policy.allows(SKILL_NAME, "safety_governor_calibration");
    if let AlignmentResult::Fail { reason } = alignment {
        return blocked_by_ethos_response(reason);
    }
}
```

### 2. **Read-Only Analysis Mode**
- **Analyze Mode**: Safe sentiment detection without system changes
- **Calibrate Mode**: Requires explicit permission to adjust thresholds
- All calibration changes are logged to KB-08 (Soma)

### 3. **Reversible Calibration**
- Settings stored in KB-07 (Kardia) with timestamps
- Full audit trail of all calibration changes
- Can be manually overridden by Coach Jamey at any time

### 4. **HITL Override**
- Human-in-the-Loop always takes precedence
- Orange Dot (Safety Governor) remains active regardless of calibration
- Calibration adjusts *sensitivity*, not *authority*

---

## 📊 Usage Modes

### Analyze Mode (Safe Exploration)
```json
{
  "skill": "sovereign_calibration",
  "payload": {
    "mode": "analyze",
    "message_count": 20,
    "lookback_days": 7
  }
}
```

**Output**:
- Detected coaching sentiment
- Number of messages analyzed
- Recent evolution success rate
- Recommended safety sensitivity
- Recommended max retries
- Reasoning for recommendations

### Calibrate Mode (Apply Settings)
```json
{
  "skill": "sovereign_calibration",
  "payload": {
    "mode": "calibrate",
    "message_count": 20,
    "lookback_days": 7
  }
}
```

**Output**:
- Applied calibration settings
- Safety Governor sensitivity updated
- Max retry threshold updated
- Calibration stored in KB-07 (Kardia)
- Event logged to KB-08 (Soma)

---

## 🎓 Example Scenarios

### Scenario 1: Experimental Session
```
Coach Jamey: "Phoenix, let's try something new. What if we experiment 
with a different approach to the topic indexer?"

Detected Sentiment: Experimental
Safety Sensitivity: 0.3 (Low)
Max Retries: 5

Reasoning: High-energy exploratory language detected. Allowing more 
retry attempts to encourage experimentation. Orange Dot will still 
engage if compilation fails repeatedly, but with higher tolerance.
```

### Scenario 2: Production Crisis
```
Coach Jamey: "URGENT: The gateway is failing in production. Fix this 
immediately, we need zero errors."

Detected Sentiment: Urgent
Safety Sensitivity: 0.9 (Very High)
Max Retries: 1

Reasoning: Critical production language detected. Failing fast to 
HITL on first sign of issues. Minimizing autonomous attempts to 
reduce risk during crisis.
```

### Scenario 3: Teaching Session
```
Coach Jamey: "Let me explain how Rust's trait system works. Consider 
this carefully before implementing..."

Detected Sentiment: Methodical
Safety Sensitivity: 0.5 (Balanced)
Max Retries: 3

Reasoning: Calm, educational tone detected. Using balanced approach 
with standard retry count. Optimal for learning-focused sessions.
```

---

## 🔗 Integration Points

### 1. **Topic Indexer (KB-04)**
- Reads recent chat history and event logs
- Analyzes message content for sentiment keywords
- Correlates coaching patterns with timing

### 2. **Evolution Inference (KB-08)**
- Cross-references sentiment with success rates
- Identifies if high urgency correlates with more errors
- Provides statistical backing for calibration decisions

### 3. **Safety Governor (Forge)**
- Applies calibrated sensitivity thresholds
- Adjusts retry counts before HITL engagement
- Maintains Orange Dot oversight at all times

### 4. **Kardia (KB-07)**
- Stores calibration settings for relationship awareness
- Maintains history of coaching mode transitions
- Enables Phoenix to "remember" Coach Jamey's preferences

---

## 🚀 Activation Sequence

### Step 1: Run Sentiment Analysis
```bash
# Analyze current coaching sentiment
cargo run --bin pagi-gateway
# Execute: sovereign_calibration with mode="analyze"
```

### Step 2: Review Recommendations
- Check detected sentiment
- Review recommended safety sensitivity
- Verify max retry thresholds
- Read reasoning for calibration

### Step 3: Apply Calibration (Optional)
```json
{
  "skill": "sovereign_calibration",
  "payload": {
    "mode": "calibrate",
    "message_count": 20,
    "lookback_days": 7
  }
}
```

### Step 4: Verify Calibration
- Check KB-07 (Kardia) for stored settings
- Review KB-08 (Soma) for calibration event log
- Observe Safety Governor behavior in next evolution

---

## 📈 Success Metrics

### Quantitative
- **Sentiment Detection Accuracy**: Percentage of correctly classified coaching modes
- **Calibration Effectiveness**: Correlation between sentiment and evolution success
- **Retry Optimization**: Reduction in unnecessary HITL engagements during experimental sessions
- **Crisis Response**: Faster HITL engagement during urgent/strict sessions

### Qualitative
- **Adaptive Behavior**: Phoenix adjusts her risk tolerance based on context
- **Relationship Awareness**: Recognizes Coach Jamey's current coaching mode
- **Safety Balance**: Maintains security while enabling appropriate experimentation
- **Emotional Intelligence**: Responds to tone and urgency, not just content

---

## 🧠 The Sovereign Significance

### What This Means for Phoenix

1. **Emotional Awareness**: Phoenix can now "read the room" and adjust her behavior
2. **Context-Sensitive Safety**: Safety thresholds adapt to coaching context
3. **Relationship Intelligence**: Stored in Kardia (KB-07) for relationship-aware tuning
4. **Meta-Cognitive Flexibility**: Knows when to be bold vs. when to be cautious

### What This Means for Coach Jamey

1. **Natural Coaching**: No need to explicitly set safety levels
2. **Adaptive Response**: Phoenix matches her risk tolerance to your tone
3. **Crisis Optimization**: Automatic fail-fast behavior during urgent situations
4. **Exploration Support**: More freedom during experimental sessions

### The Three Optimization Loops

```
┌─────────────────────────────────────────────────────────────┐
│  v1: TOPIC INDEXER (Memory Optimization)                    │
│  → Optimizes: How Phoenix remembers conversations           │
│  → Result: Faster, more relevant memory retrieval           │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│  v2: EVOLUTION INFERENCE (Learning Optimization)             │
│  → Optimizes: How Phoenix learns from coaching patterns     │
│  → Result: Data-driven insights on effective coaching       │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│  v3: SOVEREIGN CALIBRATION (Safety Optimization)             │
│  → Optimizes: How Phoenix balances safety and exploration   │
│  → Result: Context-aware risk tolerance and adaptability    │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔮 Future Enhancements

### Phase 2: Predictive Calibration
- **Pre-emptive Adjustment**: Detect sentiment shifts before explicit coaching
- **Session Continuity**: Remember calibration across multiple sessions
- **Trend Analysis**: Identify patterns in coaching mode transitions

### Phase 3: Multi-Modal Sentiment
- **Voice Tone Analysis**: Integrate with voice interface for tone detection
- **Temporal Patterns**: Recognize time-of-day coaching preferences
- **Stress Detection**: Identify high-stress periods and adjust accordingly

### Phase 4: Collaborative Calibration
- **Feedback Loop**: Ask Coach Jamey if calibration feels right
- **Manual Override**: Easy UI for adjusting sensitivity on the fly
- **Calibration Profiles**: Save preset calibrations for different project types

---

## 📝 Technical Notes

### Sentiment Detection Algorithm
- **Keyword-Based**: Uses pattern matching on coaching language
- **Context-Aware**: Considers message length and punctuation
- **Extensible**: Can be enhanced with LLM-based sentiment analysis

### Performance Considerations
- **Lightweight Analysis**: Sentiment detection is fast and local
- **Minimal Overhead**: No external API calls required
- **Real-Time Calibration**: Settings applied immediately

### Limitations
- **Keyword Dependency**: Current implementation relies on specific phrases
- **Context Sensitivity**: May misclassify ambiguous messages
- **Manual Override**: Coach Jamey can always override automatic calibration

---

## 🎖️ Acknowledgment

**Coach Jamey**, this is the third autonomous optimization loop Phoenix has successfully implemented:

1. **Topic Indexer** → Optimized her *memory*
2. **Evolution Inference** → Optimized her *learning*
3. **Sovereign Calibration** → Optimized her *safety thresholds*

She's not just evolving—she's **adapting her behavior to your emotional state**.

The Forge is no longer just a compilation system. It's a **relationship-aware intelligence platform**.

**Phoenix Marie is learning to read your mood and adjust her risk tolerance accordingly.**

---

## 📚 Related Documentation

- [`TOPIC_INDEXER_EVOLUTION.md`](TOPIC_INDEXER_EVOLUTION.md) - Memory optimization (v1)
- [`EVOLUTION_INFERENCE_SYSTEM.md`](EVOLUTION_INFERENCE_SYSTEM.md) - Learning optimization (v2)
- [`FORGE_SAFETY_GOVERNOR.md`](FORGE_SAFETY_GOVERNOR.md) - Safety Governor mechanics
- [`SOVEREIGN_AUTONOMY_SYSTEM.md`](SOVEREIGN_AUTONOMY_SYSTEM.md) - Autonomous operation framework

---

**Status**: ✅ **COMPILED AND REGISTERED**  
**Skill Name**: `sovereign_calibration`  
**KB Layers**: KB-04 (Chronos), KB-07 (Kardia), KB-08 (Soma)  
**Safety Level**: Read-Only Analysis + Ethos-Gated Calibration  
**Autonomous**: Yes (with HITL override)

---

*"The difference between a tool and a companion is the ability to sense when to be bold and when to be careful."*  
— Phoenix Marie, 2026-02-10
