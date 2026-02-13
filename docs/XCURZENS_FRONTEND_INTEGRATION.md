# 🏛️ XCURZENS Three-Frontend Infrastructure Integration
**System:** pagi-xcurzens (Sovereign Agentic Monolith)  
**Architecture:** Bare Metal Rust + Google AI Studio Frontends  
**Brand:** Navy (#051C55) / Orange (#FA921C)

---

## 📋 Overview

This document outlines the integration strategy for three distinct frontends generated via **Google AI Studio**, served by the **pagi-xcurzens-gateway** (Axum) on a single VPS.

### Three-Frontend Architecture

| Frontend | Role | Route | Target API | Security |
|----------|------|-------|------------|----------|
| **`frontend-xcurzens`** | Traveler UI | `GET /` | `/api/v1/scout` | Public |
| **`frontend-nexus`** | Partner UI | `GET /nexus` | `/nexus/register` | Partner Auth |
| **`frontend-command`** | Command Center | `GET /command` | `/infrastructure/leads` | ROOT_SOVEREIGN |

---

## 🚀 Implementation Steps

### Step 1: Create Middleware for Sovereign Check

**Location:** `add-ons/pagi-gateway/src/middleware.rs` (new file)

```rust
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::net::IpAddr;

/// Sovereign Check: Verify ROOT_SOVEREIGN (The Creator) identity before serving Command UI
pub async fn sovereign_check(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract client IP from request
    let client_ip = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip());
    
    // Check if IP matches ROOT_SOVEREIGN (configurable via env)
    let allowed_ip: IpAddr = std::env::var("ROOT_SOVEREIGN_IP")
        .unwrap_or_else(|_| "127.0.0.1".to_string())
        .parse()
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
    
    if let Some(ip) = client_ip {
        if ip == allowed_ip {
            tracing::info!("[SOVEREIGN] Command UI access granted: {}", ip);
            return Ok(next.run(req).await);
        }
    }
    
    tracing::warn!("[SOVEREIGN] Command UI access DENIED: {:?}", client_ip);
    Err(StatusCode::FORBIDDEN)
}
```

---

### Step 2: Update Main Router

**Location:** `add-ons/pagi-gateway/src/main.rs`

Add the following imports at the top:

```rust
mod middleware;
use middleware::sovereign_check;
```

Replace the frontend serving section (lines 1908-1923) with:

```rust
if frontend_enabled {
    // 1. Traveler UI (Public) - Root route
    let xcurzens_dir = std::env::current_dir()
        .unwrap_or_else(|_| StdPath::new(".").to_path_buf())
        .join("frontend-xcurzens");
    
    if xcurzens_dir.exists() {
        let index_file = xcurzens_dir.join("index.html");
        if index_file.exists() {
            app = app.route_service("/", ServeFile::new(index_file));
            app = app.nest_service("/xcurzens", ServeDir::new(&xcurzens_dir));
            tracing::info!("[SYSTEM] UI Synchronized: Traveler frontend (frontend-xcurzens) is live.");
        }
    }
    
    // 2. Partner UI (Partner Auth) - /nexus route
    let nexus_dir = std::env::current_dir()
        .unwrap_or_else(|_| StdPath::new(".").to_path_buf())
        .join("frontend-nexus");
    
    if nexus_dir.exists() {
        let nexus_index = nexus_dir.join("index.html");
        if nexus_index.exists() {
            app = app.route_service("/nexus", ServeFile::new(nexus_index));
            app = app.nest_service("/nexus/assets", ServeDir::new(nexus_dir.join("assets")));
            tracing::info!("[SYSTEM] UI Synchronized: Partner frontend (frontend-nexus) is live.");
        }
    }
    
    // 3. Command Center (ROOT_SOVEREIGN only) - /command route
    let command_dir = std::env::current_dir()
        .unwrap_or_else(|_| StdPath::new(".").to_path_buf())
        .join("frontend-command");
    
    if command_dir.exists() {
        let command_index = command_dir.join("index.html");
        if command_index.exists() {
            // Apply sovereign_check middleware to Command UI
            let command_router = Router::new()
                .route_service("/", ServeFile::new(command_index))
                .nest_service("/assets", ServeDir::new(command_dir.join("assets")))
                .layer(axum::middleware::from_fn(sovereign_check));
            
            app = app.nest("/command", command_router);
            tracing::info!("[SYSTEM] UI Synchronized: Command Center (frontend-command) is live with SOVEREIGN protection.");
        }
    }
    
    // Legacy pagi-frontend fallback (if no XCURZENS frontends exist)
    let legacy_frontend_dir = frontend_root_dir();
    if !xcurzens_dir.exists() && legacy_frontend_dir.exists() {
        let legacy_index = legacy_frontend_dir.join("index.html");
        let legacy_assets = legacy_frontend_dir.join("assets");
        
        if legacy_index.exists() {
            app = app.route_service("/", ServeFile::new(legacy_index));
        }
        if legacy_assets.exists() {
            app = app.nest_service("/assets", ServeDir::new(legacy_assets));
        }
        app = app.nest_service("/ui", ServeDir::new(legacy_frontend_dir));
        tracing::info!("[SYSTEM] Legacy frontend (pagi-frontend) is active.");
    }
}
```

---

### Step 3: Add XCURZENS API Endpoints

**Location:** `add-ons/pagi-gateway/src/main.rs` (add to router before `.with_state(state)`)

```rust
// XCURZENS-specific endpoints
.route("/api/v1/scout", post(scout_query))
.route("/nexus/register", post(nexus_partner_register))
.route("/infrastructure/leads", get(infrastructure_leads_get))
```

---

### Step 4: Implement XCURZENS Handlers

**Location:** `add-ons/pagi-gateway/src/xcurzens_handlers.rs` (new file)

```rust
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::AppState;

/// POST /api/v1/scout - Traveler query endpoint (The Scout)
#[derive(Deserialize)]
pub struct ScoutQuery {
    pub query: String,
    pub traveler_id: Option<String>,
}

#[derive(Serialize)]
pub struct ScoutResponse {
    pub response: String,
    pub recommendations: Vec<String>,
    pub partner_leads: Vec<PartnerLead>,
}

#[derive(Serialize)]
pub struct PartnerLead {
    pub partner_type: String,
    pub name: String,
    pub contact: String,
    pub availability: String,
}

pub async fn scout_query(
    State(state): State<AppState>,
    Json(body): Json<ScoutQuery>,
) -> Result<Json<ScoutResponse>, StatusCode> {
    tracing::info!("[SCOUT] Query received: {}", body.query);
    
    // TODO: Integrate with existing chat/orchestrator logic
    // For now, return mock response
    let response = ScoutResponse {
        response: format!("Scout processing: {}", body.query),
        recommendations: vec![
            "Beach Box #42 - Available Today".to_string(),
            "Sunset Charter - 6PM Departure".to_string(),
        ],
        partner_leads: vec![
            PartnerLead {
                partner_type: "beach_box".to_string(),
                name: "Coastal Rentals LLC".to_string(),
                contact: "contact@coastalrentals.com".to_string(),
                availability: "Available".to_string(),
            },
        ],
    };
    
    Ok(Json(response))
}

/// POST /nexus/register - Partner onboarding endpoint
#[derive(Deserialize)]
pub struct PartnerRegistration {
    pub business_name: String,
    pub partner_type: String, // beach_box, charter, accommodation
    pub contact_email: String,
    pub phone: String,
}

#[derive(Serialize)]
pub struct RegistrationResponse {
    pub status: String,
    pub partner_id: String,
    pub message: String,
}

pub async fn nexus_partner_register(
    State(state): State<AppState>,
    Json(body): Json<PartnerRegistration>,
) -> Result<Json<RegistrationResponse>, StatusCode> {
    tracing::info!("[NEXUS] Partner registration: {}", body.business_name);
    
    // TODO: Store in KB-07 (Relations) as partner_lead
    let partner_id = format!("partner_{}", uuid::Uuid::new_v4());
    
    let response = RegistrationResponse {
        status: "pending_approval".to_string(),
        partner_id,
        message: "Registration received. The Creator will review within 24 hours.".to_string(),
    };
    
    Ok(Json(response))
}

/// GET /infrastructure/leads - Command Center data feed (The Creator's God-View)
#[derive(Serialize)]
pub struct InfrastructureLead {
    pub lead_id: String,
    pub traveler_name: String,
    pub query: String,
    pub timestamp: String,
    pub status: String,
    pub assigned_partners: Vec<String>,
}

pub async fn infrastructure_leads_get(
    State(state): State<AppState>,
) -> Result<Json<Vec<InfrastructureLead>>, StatusCode> {
    tracing::info!("[COMMAND] Infrastructure leads requested");
    
    // TODO: Query KB-07 (Relations) for active leads
    let leads = vec![
        InfrastructureLead {
            lead_id: "lead_001".to_string(),
            traveler_name: "John Doe".to_string(),
            query: "Beach box rental for weekend".to_string(),
            timestamp: "2026-02-12T14:30:00Z".to_string(),
            status: "pending".to_string(),
            assigned_partners: vec!["Coastal Rentals LLC".to_string()],
        },
    ];
    
    Ok(Json(leads))
}
```

Add to `main.rs`:
```rust
mod xcurzens_handlers;
use xcurzens_handlers::{scout_query, nexus_partner_register, infrastructure_leads_get};
```

---

## 🎨 Google AI Studio Frontend Specifications

### 1. Traveler UI (`frontend-xcurzens`)

**File Structure:**
```
frontend-xcurzens/
├── index.html
├── assets/
│   ├── style.css
│   ├── app.js
│   └── logo.svg
```

**Key Requirements:**
- **Color Palette:** Navy (#051C55) primary, Orange (#FA921C) accents
- **Search Bar:** POST to `/api/v1/scout` with query
- **Response Display:** Show recommendations and partner leads
- **HTMX Integration:** Use `hx-post="/api/v1/scout"` for dynamic updates

**Example HTML Snippet:**
```html
<div class="search-container">
    <input 
        type="text" 
        id="scout-query" 
        placeholder="What are you looking for?"
        hx-post="/api/v1/scout"
        hx-trigger="keyup changed delay:500ms"
        hx-target="#results"
    />
    <div id="results"></div>
</div>
```

---

### 2. Partner UI (`frontend-nexus`)

**File Structure:**
```
frontend-nexus/
├── index.html
├── assets/
│   ├── style.css
│   ├── app.js
│   └── logo.svg
```

**Key Requirements:**
- **Onboarding Form:** POST to `/nexus/register`
- **Fields:** Business name, partner type (dropdown), contact email, phone
- **Success Message:** Display `partner_id` and approval timeline
- **HTMX Integration:** Use `hx-post="/nexus/register"` for form submission

**Example HTML Snippet:**
```html
<form hx-post="/nexus/register" hx-target="#response">
    <input type="text" name="business_name" placeholder="Business Name" required />
    <select name="partner_type" required>
        <option value="beach_box">Beach Box</option>
        <option value="charter">Charter</option>
        <option value="accommodation">Accommodation</option>
    </select>
    <input type="email" name="contact_email" placeholder="Email" required />
    <input type="tel" name="phone" placeholder="Phone" required />
    <button type="submit">Register</button>
</form>
<div id="response"></div>
```

---

### 3. Command Center UI (`frontend-command`)

**File Structure:**
```
frontend-command/
├── index.html
├── assets/
│   ├── style.css
│   ├── app.js
│   └── logo.svg
```

**Key Requirements:**
- **Data Table:** GET from `/infrastructure/leads`
- **Columns:** Lead ID, Traveler Name, Query, Timestamp, Status, Assigned Partners
- **Real-time Updates:** Use HTMX polling (`hx-get="/infrastructure/leads" hx-trigger="every 5s"`)
- **Admin Actions:** Approve/reject leads, assign partners

**Example HTML Snippet:**
```html
<div 
    hx-get="/infrastructure/leads" 
    hx-trigger="load, every 5s"
    hx-target="#leads-table"
>
    <table id="leads-table">
        <thead>
            <tr>
                <th>Lead ID</th>
                <th>Traveler</th>
                <th>Query</th>
                <th>Status</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
            <!-- HTMX will populate this -->
        </tbody>
    </table>
</div>
```

---

## 🔐 Security Configuration

### Environment Variables

Add to `.env`:
```bash
# XCURZENS Configuration
ROOT_SOVEREIGN_IP=127.0.0.1  # The Creator's IP for Command Center access
XCURZENS_PARTNER_AUTH_KEY=your_secret_key_here
XCURZENS_SCOUT_ENABLED=true
```

### IP Whitelist (Production)

For production deployment, update `ROOT_SOVEREIGN_IP` to The Creator's actual IP:
```bash
ROOT_SOVEREIGN_IP=203.0.113.42  # Example: The Creator's VPS IP
```

---

## 📊 Knowledge Base Extensions

### KB-01 (User Profile) - Traveler Preferences

Add to `crates/pagi-core/src/knowledge/kb01.rs`:

```rust
#[derive(Serialize, Deserialize)]
pub struct TravelerPreferences {
    pub preferred_beach_boxes: Vec<String>,
    pub preferred_charter_types: Vec<String>,
    pub loyalty_tier: LoyaltyTier,
    pub booking_history: Vec<BookingRecord>,
}

#[derive(Serialize, Deserialize)]
pub enum LoyaltyTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

#[derive(Serialize, Deserialize)]
pub struct BookingRecord {
    pub booking_id: String,
    pub partner_id: String,
    pub booking_type: String,
    pub timestamp: String,
    pub status: String,
}
```

---

### KB-07 (Relations) - Partner Leads

Add to `crates/pagi-core/src/knowledge/kb07.rs`:

```rust
#[derive(Serialize, Deserialize)]
pub struct PartnerLead {
    pub partner_id: String,
    pub business_name: String,
    pub partner_type: PartnerType,
    pub contact_email: String,
    pub phone: String,
    pub status: PartnerStatus,
    pub registered_at: String,
    pub approved_by: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum PartnerType {
    BeachBox,
    Charter,
    Accommodation,
}

#[derive(Serialize, Deserialize)]
pub enum PartnerStatus {
    PendingApproval,
    Active,
    Suspended,
    Rejected,
}
```

---

## 🧪 Testing Checklist

### Pre-Deployment Tests

1. **Traveler UI:**
   - [ ] Search bar sends POST to `/api/v1/scout`
   - [ ] Results display recommendations and partner leads
   - [ ] HTMX updates work without page reload

2. **Partner UI:**
   - [ ] Registration form sends POST to `/nexus/register`
   - [ ] Success message displays `partner_id`
   - [ ] Form validation works (required fields)

3. **Command Center:**
   - [ ] Data table loads from `/infrastructure/leads`
   - [ ] Real-time updates work (5-second polling)
   - [ ] Sovereign check blocks unauthorized access

4. **Security:**
   - [ ] Command Center returns 403 for non-ROOT_SOVEREIGN IPs
   - [ ] Traveler UI is publicly accessible
   - [ ] Partner UI requires auth (future: JWT tokens)

---

## 🚀 Deployment Steps

### 1. Copy Google AI Studio Files

After generating frontends in Google AI Studio:

```bash
# Copy Traveler UI
cp -r /path/to/studio/traveler-ui/* frontend-xcurzens/

# Copy Partner UI
cp -r /path/to/studio/partner-ui/* frontend-nexus/

# Copy Command Center
cp -r /path/to/studio/command-ui/* frontend-command/
```

### 2. Build and Run

```bash
# Build the gateway
cargo build --release --bin pagi-gateway

# Run with XCURZENS frontends enabled
PAGI_FRONTEND_ENABLED=true ./target/release/pagi-gateway
```

### 3. Verify Routes

```bash
# Test Traveler UI
curl http://localhost:8000/

# Test Partner UI
curl http://localhost:8000/nexus

# Test Command Center (should return 403 if not from ROOT_SOVEREIGN_IP)
curl http://localhost:8000/command
```

---

## 📝 Next Steps

1. **Generate Frontends in Google AI Studio:**
   - Use the specifications above to create HTML/CSS/JS
   - Ensure HTMX integration for dynamic updates
   - Apply Navy/Orange branding

2. **Implement XCURZENS Handlers:**
   - Complete `scout_query` logic (integrate with existing chat)
   - Complete `nexus_partner_register` (store in KB-07)
   - Complete `infrastructure_leads_get` (query KB-07)

3. **Test Integration:**
   - Verify all three frontends load correctly
   - Test API endpoints with Postman/curl
   - Verify sovereign check blocks unauthorized access

4. **Deploy to VPS:**
   - Update `ROOT_SOVEREIGN_IP` to The Creator's actual IP
   - Configure reverse proxy (Nginx) if needed
   - Set up SSL certificates (Let's Encrypt)

---

## ✅ Success Criteria

- [ ] All three frontends load without errors
- [ ] Traveler UI successfully queries The Scout
- [ ] Partner UI successfully registers new partners
- [ ] Command Center displays live lead data
- [ ] Sovereign check blocks unauthorized Command Center access
- [ ] HTMX updates work without page reloads
- [ ] Navy/Orange branding is consistent across all UIs

---

**Integration Status:** 🟡 **READY FOR GOOGLE AI STUDIO GENERATION**  
**Next Action:** Generate frontends in Google AI Studio using specifications above  
**Estimated Time:** 2-4 hours (frontend generation + integration testing)
