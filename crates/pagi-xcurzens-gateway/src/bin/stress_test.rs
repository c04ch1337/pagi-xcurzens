//! System Stress Test — Validates Sled and NEXUS Bridge under load.
//! Simulates "Busy Saturday in Corpus Christi": 10 concurrent travelers, 50 Scout requests.
//! Run with gateway up: cargo run --bin stress_test

use reqwest::Client;
use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

const BASE_URL: &str = "http://127.0.0.1:8000";
const CONCURRENT_TRAVELERS: usize = 10;
const REQUESTS_PER_TRAVELER: usize = 5;

// Mix of high-intent (trigger LeadDispatcher) and normal queries.
const QUERIES: &[&str] = &[
    "What's the price for a 2-hour charter?",
    "Do you have availability this weekend for a beach box?",
    "I want to book a boat rental for next Saturday.",
    "How much does a sunset cruise cost?",
    "Can I reserve a Beach Box location for next week?",
    "What's the weather like on the water today?",
    "Tell me about coastal activities.",
    "Is it safe to go out with this wind?",
    "Get me a quote for a group charter.",
    "Any availability for rentals tomorrow?",
];

const CITIES: &[&str] = &["Corpus Christi", "Galveston", "Port Aransas", "South Padre", "Rockport"];
const WEATHER: &[&str] = &["Sunny, 78°F", "Choppy, 72°F", "Clear, 82°F", "Breezy, 75°F", "Incoming storm"];

#[tokio::main]
async fn main() {
    println!("[STRESS TEST] Starting — {} travelers × {} requests = {} total", CONCURRENT_TRAVELERS, REQUESTS_PER_TRAVELER, CONCURRENT_TRAVELERS * REQUESTS_PER_TRAVELER);
    println!("[STRESS TEST] Target: {} (ensure gateway is running)", BASE_URL);

    let success = Arc::new(AtomicU32::new(0));
    let failure = Arc::new(AtomicU32::new(0));
    let latencies: Arc<RwLock<Vec<u64>>> = Arc::new(RwLock::new(Vec::new()));

    let client = Client::new();

    let mut handles = Vec::new();
    for traveler_id in 0..CONCURRENT_TRAVELERS {
        let client = client.clone();
        let success = Arc::clone(&success);
        let failure = Arc::clone(&failure);
        let latencies = Arc::clone(&latencies);

        let h = tokio::spawn(async move {
            for r in 0..REQUESTS_PER_TRAVELER {
                let q_idx = (traveler_id + r) % QUERIES.len();
                let c_idx = (traveler_id + r) % CITIES.len();
                let w_idx = (traveler_id + r) % WEATHER.len();

                let body = json!({
                    "query": QUERIES[q_idx],
                    "city": CITIES[c_idx],
                    "weather": WEATHER[w_idx],
                });

                let start = Instant::now();
                let res = client
                    .post(format!("{}/api/v1/scout", BASE_URL))
                    .json(&body)
                    .send()
                    .await;
                let elapsed_ms = start.elapsed().as_millis() as u64;

                if let Ok(resp) = res {
                    if resp.status().is_success() {
                        success.fetch_add(1, Ordering::Relaxed);
                        latencies.write().await.push(elapsed_ms);
                    } else {
                        failure.fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    failure.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
        handles.push(h);
    }

    for h in handles {
        let _ = h.await;
    }

    let s = success.load(Ordering::Relaxed);
    let f = failure.load(Ordering::Relaxed);
    let total = s + f;
    let success_rate = if total > 0 { (s as f64 / total as f64) * 100.0 } else { 0.0 };
    let latencies_guard = latencies.read().await;
    let avg_latency_ms = if latencies_guard.is_empty() {
        0.0
    } else {
        latencies_guard.iter().sum::<u64>() as f64 / latencies_guard.len() as f64
    };

    println!(
        "[STRESS TEST] Bandwidth Capacity: {:.1}% | Average Latency: {:.0}ms",
        success_rate, avg_latency_ms
    );
    println!(
        "[STRESS TEST] Total: {} | Success: {} | Failure: {}",
        total, s, f
    );
    println!("[STRESS TEST] Jamey's Check: View /command dashboard to confirm leads and high-intent flags in KB-07.");
}
