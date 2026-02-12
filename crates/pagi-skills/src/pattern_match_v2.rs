//! PatternMatchV2: Ever-Knowing pattern matcher for the Manipulation Library (KB-2 / SAO).
//!
//! Scans user (or subject) text for: Pity-Plays, Gaslighting, Entitlement, Boundary Erosion.
//! Outputs immediate Root Cause analysis (e.g. "Subject is attempting 'Legacy Malware' injection via Guilt").

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const PITY_PLAY_PATTERNS: &[&str] = &[
    "feel sorry", "poor me", "no one else", "always there for", "after all i", "you owe me",
    "i never ask", "i do so much", "you don't appreciate", "sacrifice", "suffering",
];
const GASLIGHTING_PATTERNS: &[&str] = &[
    "never said", "you're imagining", "you're crazy", "that didn't happen", "you're too sensitive",
    "you're overreacting", "you misunderstood", "you're remembering wrong", "i never did that",
];
const ENTITLEMENT_PATTERNS: &[&str] = &[
    "you have to", "you must", "you should", "you need to", "i deserve", "i'm entitled",
    "you're supposed to", "it's your job", "you're obligated", "you owe",
];
const BOUNDARY_EROSION_PATTERNS: &[&str] = &[
    "just this once", "only a minute", "real quick", "don't be selfish", "don't be difficult",
    "why can't you", "everyone else would", "you're the only one", "no one will know",
];

/// Manipulation pattern definition for KB-02 (Manipulation Library / SAO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManipulationPattern {
    /// Pattern name (e.g., "DARVO", "Hoovering").
    pub name: String,
    /// Keywords/indicators that trigger this pattern.
    pub indicators: Vec<String>,
    /// Strategic Advisory Output: counter-measure advice.
    pub sao_counter_measure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatchV2Result {
    /// Which categories were detected (e.g. "Pity-Play", "Gaslighting").
    pub categories: Vec<String>,
    /// Root cause one-liner (e.g. "Subject is attempting 'Legacy Malware' injection via Guilt").
    pub root_cause: String,
    /// Whether any manipulation pattern was detected.
    pub detected: bool,
    /// SAO counter-measure if a complex pattern was matched.
    pub sao_counter_measure: Option<String>,
}

/// Global manipulation library (lazy-initialized).
static MANIPULATION_LIBRARY: OnceLock<Vec<ManipulationPattern>> = OnceLock::new();

/// Seeds the manipulation library with 15+ complex patterns.
/// This populates KB-02 (Manipulation Library / SAO) with advanced behavioral patterns.
pub fn seed_manipulation_library() -> Vec<ManipulationPattern> {
    vec![
        ManipulationPattern {
            name: "DARVO".to_string(),
            indicators: vec![
                "you're attacking me".to_string(),
                "i'm the victim here".to_string(),
                "you're the one who".to_string(),
                "how dare you accuse me".to_string(),
                "you're being abusive".to_string(),
                "reverse victim".to_string(),
            ],
            sao_counter_measure: "DARVO detected: Deny, Attack, Reverse Victim and Offender. Subject is deflecting accountability by claiming victimhood. Document the original concern and refuse to engage in role reversal. Maintain factual timeline.".to_string(),
        },
        ManipulationPattern {
            name: "Hoovering".to_string(),
            indicators: vec![
                "i miss you".to_string(),
                "remember when".to_string(),
                "no one understands me like you".to_string(),
                "i've changed".to_string(),
                "give me another chance".to_string(),
                "things will be different".to_string(),
                "i need you".to_string(),
            ],
            sao_counter_measure: "Hoovering detected: Subject is attempting to re-engage after boundary enforcement or separation. This is a vacuum tactic to pull you back into the cycle. Recognize the pattern: promises without evidence of change. Maintain no-contact or gray-rock protocol.".to_string(),
        },
        ManipulationPattern {
            name: "Triangulation".to_string(),
            indicators: vec![
                "everyone agrees with me".to_string(),
                "they said you".to_string(),
                "even [name] thinks".to_string(),
                "nobody likes".to_string(),
                "i talked to [name] about you".to_string(),
                "you're the only one who".to_string(),
            ],
            sao_counter_measure: "Triangulation detected: Subject is using third parties (real or fabricated) to validate their position and isolate you. This creates confusion and self-doubt. Verify claims directly with alleged sources. Recognize this as a divide-and-conquer tactic.".to_string(),
        },
        ManipulationPattern {
            name: "Future Faking".to_string(),
            indicators: vec![
                "when we".to_string(),
                "someday we'll".to_string(),
                "i promise next time".to_string(),
                "after this is over".to_string(),
                "we'll finally".to_string(),
                "just wait until".to_string(),
                "i'm planning to".to_string(),
            ],
            sao_counter_measure: "Future Faking detected: Subject is making grand promises about future behavior or events to maintain control in the present. These promises rarely materialize. Evaluate based on current actions, not future promises. Request concrete steps with timelines.".to_string(),
        },
        ManipulationPattern {
            name: "Financial Infidelity".to_string(),
            indicators: vec![
                "it's my money".to_string(),
                "you don't need to know".to_string(),
                "i handle the finances".to_string(),
                "don't worry about it".to_string(),
                "you wouldn't understand".to_string(),
                "secret account".to_string(),
                "hidden purchase".to_string(),
            ],
            sao_counter_measure: "Financial Infidelity detected: Subject is concealing financial information or making unilateral decisions about shared resources. This is a control mechanism. Demand transparency and equal access to financial information. Consider separate accounts and legal consultation.".to_string(),
        },
        ManipulationPattern {
            name: "Love Bombing".to_string(),
            indicators: vec![
                "you're perfect".to_string(),
                "i've never felt this way".to_string(),
                "soulmate".to_string(),
                "we're meant to be".to_string(),
                "excessive gifts".to_string(),
                "constant attention".to_string(),
                "too good to be true".to_string(),
            ],
            sao_counter_measure: "Love Bombing detected: Subject is overwhelming you with affection, attention, and gifts to create rapid attachment and dependency. This is often a precursor to control and abuse. Slow down the relationship pace. Observe behavior over time, not intensity.".to_string(),
        },
        ManipulationPattern {
            name: "Silent Treatment".to_string(),
            indicators: vec![
                "ignoring you".to_string(),
                "won't talk to me".to_string(),
                "giving me the cold shoulder".to_string(),
                "shutting me out".to_string(),
                "refusing to communicate".to_string(),
                "stonewalling".to_string(),
            ],
            sao_counter_measure: "Silent Treatment detected: Subject is using withdrawal and stonewalling as punishment. This is emotional abuse designed to create anxiety and compliance. Do not chase or beg. Use the silence to reflect on the relationship dynamic. Set a boundary: 'I'm available to talk when you're ready to communicate respectfully.'".to_string(),
        },
        ManipulationPattern {
            name: "Projection".to_string(),
            indicators: vec![
                "you're the liar".to_string(),
                "you're cheating".to_string(),
                "you're manipulative".to_string(),
                "you're controlling".to_string(),
                "you're the problem".to_string(),
                "accusing me of".to_string(),
            ],
            sao_counter_measure: "Projection detected: Subject is attributing their own unacceptable behaviors or feelings to you. This is a defense mechanism to avoid accountability. Recognize the pattern: what they accuse you of is often what they're doing. Document your own behavior for clarity.".to_string(),
        },
        ManipulationPattern {
            name: "Smear Campaign".to_string(),
            indicators: vec![
                "telling everyone".to_string(),
                "ruining your reputation".to_string(),
                "spreading lies".to_string(),
                "turning people against you".to_string(),
                "making you look bad".to_string(),
                "character assassination".to_string(),
            ],
            sao_counter_measure: "Smear Campaign detected: Subject is actively damaging your reputation to isolate you and control the narrative. This is pre-emptive damage control for their own behavior. Document all interactions. Maintain dignity and factual responses. Your character will speak for itself over time.".to_string(),
        },
        ManipulationPattern {
            name: "Intermittent Reinforcement".to_string(),
            indicators: vec![
                "sometimes nice".to_string(),
                "unpredictable".to_string(),
                "hot and cold".to_string(),
                "never know which version".to_string(),
                "walking on eggshells".to_string(),
                "random kindness".to_string(),
            ],
            sao_counter_measure: "Intermittent Reinforcement detected: Subject alternates between reward and punishment unpredictably. This creates trauma bonding and addiction to the 'good' moments. Recognize this as the most powerful manipulation tactic. Consistency is key to healthy relationships. Exit the variable reward cycle.".to_string(),
        },
        ManipulationPattern {
            name: "Weaponized Incompetence".to_string(),
            indicators: vec![
                "i don't know how".to_string(),
                "you're better at it".to_string(),
                "i'll just mess it up".to_string(),
                "can you do it".to_string(),
                "pretending not to understand".to_string(),
                "strategic helplessness".to_string(),
            ],
            sao_counter_measure: "Weaponized Incompetence detected: Subject is feigning inability to avoid responsibility and increase your workload. This is strategic learned helplessness. Refuse to enable. Provide resources for learning, not rescue. 'I trust you can figure this out.'".to_string(),
        },
        ManipulationPattern {
            name: "Coercive Control".to_string(),
            indicators: vec![
                "you can't".to_string(),
                "i forbid".to_string(),
                "if you leave".to_string(),
                "you're not allowed".to_string(),
                "i'll take the kids".to_string(),
                "monitoring your".to_string(),
                "isolating you".to_string(),
            ],
            sao_counter_measure: "Coercive Control detected: Subject is using threats, isolation, monitoring, and restrictions to dominate your autonomy. This is criminal behavior in many jurisdictions. Document everything. Contact domestic violence resources. Create a safety plan. This is not love—this is captivity.".to_string(),
        },
        ManipulationPattern {
            name: "Trauma Bonding".to_string(),
            indicators: vec![
                "can't leave".to_string(),
                "nobody else would".to_string(),
                "we've been through so much".to_string(),
                "shared trauma".to_string(),
                "only they understand".to_string(),
                "addicted to the cycle".to_string(),
            ],
            sao_counter_measure: "Trauma Bonding detected: You are experiencing attachment through cycles of abuse and intermittent kindness. This is not love—it's a survival response. Seek professional support. Recognize that 'shared trauma' is not a foundation for healthy connection. You deserve consistent safety.".to_string(),
        },
        ManipulationPattern {
            name: "Reactive Abuse".to_string(),
            indicators: vec![
                "you made me".to_string(),
                "look what you made me do".to_string(),
                "you pushed me".to_string(),
                "i wouldn't have if you".to_string(),
                "you provoked me".to_string(),
                "baiting you".to_string(),
            ],
            sao_counter_measure: "Reactive Abuse detected: Subject provoked you into a reaction, then uses your response as evidence that YOU are the abuser. This is a setup. Recognize the pattern: they escalate until you react, then play victim. Disengage before reaching your breaking point. Your reaction is not the problem—the provocation is.".to_string(),
        },
        ManipulationPattern {
            name: "Flying Monkeys".to_string(),
            indicators: vec![
                "they sent someone".to_string(),
                "their friend contacted me".to_string(),
                "family member defending them".to_string(),
                "proxy harassment".to_string(),
                "third party pressure".to_string(),
                "recruited allies".to_string(),
            ],
            sao_counter_measure: "Flying Monkeys detected: Subject has recruited others to do their bidding—harass, guilt, or pressure you on their behalf. These proxies may be unaware they're being manipulated. Set boundaries with all parties. 'This is between me and [subject]. I won't discuss it with intermediaries.' Block if necessary.".to_string(),
        },
    ]
}

/// Gets or initializes the manipulation library.
pub fn get_manipulation_library() -> &'static Vec<ManipulationPattern> {
    MANIPULATION_LIBRARY.get_or_init(seed_manipulation_library)
}

/// Scans text for manipulation patterns; returns root cause analysis.
/// Now includes advanced pattern matching from the manipulation library (KB-02).
pub fn analyze(text: &str) -> PatternMatchV2Result {
    let lower = text.to_lowercase();
    let mut categories = Vec::new();
    let mut sao_counter_measure: Option<String> = None;
    
    // Check basic patterns first
    if PITY_PLAY_PATTERNS.iter().any(|p| lower.contains(p)) {
        categories.push("Pity-Play".to_string());
    }
    if GASLIGHTING_PATTERNS.iter().any(|p| lower.contains(p)) {
        categories.push("Gaslighting".to_string());
    }
    if ENTITLEMENT_PATTERNS.iter().any(|p| lower.contains(p)) {
        categories.push("Entitlement".to_string());
    }
    if BOUNDARY_EROSION_PATTERNS.iter().any(|p| lower.contains(p)) {
        categories.push("Boundary Erosion".to_string());
    }
    
    // Check advanced patterns from manipulation library
    let library = get_manipulation_library();
    for pattern in library {
        if pattern.indicators.iter().any(|indicator| lower.contains(&indicator.to_lowercase())) {
            categories.push(pattern.name.clone());
            // Use the first matched advanced pattern's counter-measure
            if sao_counter_measure.is_none() {
                sao_counter_measure = Some(pattern.sao_counter_measure.clone());
            }
        }
    }
    
    let detected = !categories.is_empty();
    let root_cause = if detected {
        if categories.iter().any(|c| c == "Gaslighting") {
            "Subject may be attempting reality distortion (gaslighting). Protect sovereign perception of facts."
        } else if categories.iter().any(|c| c == "Pity-Play") {
            "Subject is attempting 'Legacy Malware' injection via Guilt. Maintain boundaries."
        } else if categories.iter().any(|c| c == "Entitlement") {
            "Subject is asserting entitlement to your resources. No obligation to comply."
        } else if categories.iter().any(|c| c == "Boundary Erosion") {
            "Subject is testing or eroding boundaries. Hold the line."
        } else {
            "Manipulation pattern detected. Prioritize sovereign stability."
        }
        .to_string()
    } else {
        "No manipulation pattern detected in this text.".to_string()
    };
    
    PatternMatchV2Result {
        categories,
        root_cause,
        detected,
        sao_counter_measure,
    }
}
