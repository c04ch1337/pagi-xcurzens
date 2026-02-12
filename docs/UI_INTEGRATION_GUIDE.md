# 🎨 Phoenix Beta - UI Integration Guide

## Overview

This guide documents how to integrate the beta distribution features into the Phoenix UI, enabling first-run setup and API key configuration.

---

## 🔌 New API Endpoints

### 1. Check API Key Status
**Endpoint**: `GET /api/v1/config/api-key`

**Purpose**: Detect if this is a first run and if API key is configured

**Response**:
```json
{
  "configured": true,
  "first_run": false,
  "has_user_name": true
}
```

**Usage**:
```typescript
async function checkApiKeyStatus() {
  const response = await fetch('/api/v1/config/api-key');
  const data = await response.json();
  return data;
}
```

---

### 2. Set API Key
**Endpoint**: `POST /api/v1/config/api-key`

**Purpose**: Save user's OpenRouter API key to local configuration

**Request Body**:
```json
{
  "api_key": "sk-or-v1-...",
  "llm_model": "anthropic/claude-opus-4.6",  // optional
  "llm_api_url": "https://openrouter.ai/api/v1/chat/completions",  // optional
  "user_name": "Beta Tester"  // optional
}
```

**Response**:
```json
{
  "success": true,
  "message": "API key saved successfully",
  "first_run": false
}
```

**Usage**:
```typescript
async function saveApiKey(apiKey: string, userName?: string) {
  const response = await fetch('/api/v1/config/api-key', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      api_key: apiKey,
      user_name: userName,
    }),
  });
  
  if (!response.ok) {
    const error = await response.text();
    throw new Error(error);
  }
  
  return await response.json();
}
```

---

### 3. Get User Configuration
**Endpoint**: `GET /api/v1/config/user`

**Purpose**: Retrieve user configuration (without exposing API key)

**Response**:
```json
{
  "first_run": false,
  "has_api_key": true,
  "llm_model": "anthropic/claude-opus-4.6",
  "llm_api_url": "https://openrouter.ai/api/v1/chat/completions",
  "user_name": "Beta Tester",
  "version": "0.1.0-beta.1"
}
```

**Usage**:
```typescript
async function getUserConfig() {
  const response = await fetch('/api/v1/config/user');
  return await response.json();
}
```

---

## 🚀 First-Run Experience Flow

### Step 1: Check on App Load
```typescript
// In your main App component or initialization
useEffect(() => {
  async function checkFirstRun() {
    const status = await checkApiKeyStatus();
    
    if (status.first_run || !status.configured) {
      // Show welcome/setup modal
      setShowSetupModal(true);
    }
  }
  
  checkFirstRun();
}, []);
```

---

### Step 2: Welcome Modal Component

```typescript
interface SetupModalProps {
  isOpen: boolean;
  onClose: () => void;
}

function SetupModal({ isOpen, onClose }: SetupModalProps) {
  const [apiKey, setApiKey] = useState('');
  const [userName, setUserName] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');

    try {
      await saveApiKey(apiKey, userName);
      onClose();
      // Optionally reload or show success message
    } catch (err) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={() => {}}>
      <div className="setup-modal">
        <h2>Welcome to Phoenix! 🔥</h2>
        <p>
          To get started, you'll need an OpenRouter API key.
          This key stays on YOUR machine and is never shared.
        </p>
        
        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="userName">Your Name (optional)</label>
            <input
              id="userName"
              type="text"
              value={userName}
              onChange={(e) => setUserName(e.target.value)}
              placeholder="How should Phoenix address you?"
            />
          </div>

          <div className="form-group">
            <label htmlFor="apiKey">OpenRouter API Key *</label>
            <input
              id="apiKey"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-or-v1-..."
              required
            />
            <small>
              Don't have one? <a href="https://openrouter.ai/keys" target="_blank">Get it here</a>
            </small>
          </div>

          {error && (
            <div className="error-message">{error}</div>
          )}

          <button type="submit" disabled={loading || !apiKey}>
            {loading ? 'Saving...' : 'Start Phoenix'}
          </button>
        </form>

        <div className="privacy-notice">
          <h4>🔐 Privacy Guarantee</h4>
          <ul>
            <li>Your API key is stored locally in <code>user_config.toml</code></li>
            <li>All your data stays on YOUR machine</li>
            <li>Phoenix never sends your personal data to external servers</li>
            <li>Only LLM API calls go through OpenRouter (using YOUR key)</li>
          </ul>
        </div>
      </div>
    </Modal>
  );
}
```

---

### Step 3: Settings Page Integration

Add API key management to your settings page:

```typescript
function SettingsPage() {
  const [config, setConfig] = useState(null);
  const [showApiKeyModal, setShowApiKeyModal] = useState(false);

  useEffect(() => {
    async function loadConfig() {
      const data = await getUserConfig();
      setConfig(data);
    }
    loadConfig();
  }, []);

  return (
    <div className="settings-page">
      <h2>Settings</h2>
      
      <section className="api-config">
        <h3>API Configuration</h3>
        
        <div className="config-item">
          <label>API Key Status:</label>
          <span className={config?.has_api_key ? 'status-ok' : 'status-warning'}>
            {config?.has_api_key ? '✓ Configured' : '⚠ Not Configured'}
          </span>
          <button onClick={() => setShowApiKeyModal(true)}>
            {config?.has_api_key ? 'Update' : 'Configure'}
          </button>
        </div>

        <div className="config-item">
          <label>LLM Model:</label>
          <span>{config?.llm_model || 'Default'}</span>
        </div>

        <div className="config-item">
          <label>User Name:</label>
          <span>{config?.user_name || 'Not set'}</span>
        </div>

        <div className="config-item">
          <label>Version:</label>
          <span>{config?.version || 'Unknown'}</span>
        </div>
      </section>

      {showApiKeyModal && (
        <SetupModal
          isOpen={showApiKeyModal}
          onClose={() => {
            setShowApiKeyModal(false);
            // Reload config
            getUserConfig().then(setConfig);
          }}
        />
      )}
    </div>
  );
}
```

---

## 🎨 UI/UX Recommendations

### Welcome Screen Design
- **Friendly & Welcoming**: Use warm colors and friendly language
- **Clear Privacy Message**: Emphasize local-first, privacy-focused approach
- **Simple Form**: Minimize friction - just API key and optional name
- **Help Links**: Direct link to OpenRouter for API key creation
- **Skip Option**: Allow users to configure later (but disable chat until configured)

### Visual Elements
```css
.setup-modal {
  max-width: 600px;
  padding: 2rem;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  border-radius: 12px;
}

.privacy-notice {
  margin-top: 2rem;
  padding: 1rem;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  font-size: 0.9rem;
}

.privacy-notice ul {
  list-style: none;
  padding-left: 0;
}

.privacy-notice li::before {
  content: "✓ ";
  color: #4ade80;
  font-weight: bold;
}
```

---

## 🔔 Error Handling

### Common Errors

1. **Invalid API Key**
```typescript
if (error.includes('API key cannot be empty')) {
  setError('Please enter a valid API key');
}
```

2. **Network Error**
```typescript
if (error.includes('Failed to fetch')) {
  setError('Unable to connect to Phoenix. Is the server running?');
}
```

3. **Server Error**
```typescript
if (error.includes('500')) {
  setError('Server error. Please try again or check logs.');
}
```

---

## 🧪 Testing Checklist

### First-Run Flow
- [ ] Fresh install shows welcome modal
- [ ] Cannot dismiss modal without configuring
- [ ] API key validation works
- [ ] Success message after configuration
- [ ] Chat works after configuration
- [ ] Settings page shows configured status

### Settings Page
- [ ] Shows current configuration
- [ ] Can update API key
- [ ] Can update user name
- [ ] Changes persist after reload
- [ ] Version number displays correctly

### Error Cases
- [ ] Empty API key shows error
- [ ] Invalid API key shows error
- [ ] Network error shows appropriate message
- [ ] Server error shows appropriate message

---

## 📱 Mobile Considerations

### Responsive Design
```css
@media (max-width: 768px) {
  .setup-modal {
    max-width: 90vw;
    padding: 1.5rem;
  }
  
  .form-group input {
    font-size: 16px; /* Prevents zoom on iOS */
  }
}
```

### Touch Targets
- Ensure buttons are at least 44x44px
- Add adequate spacing between interactive elements
- Use larger font sizes for readability

---

## 🔄 Update Flow (Future)

When auto-update is implemented:

```typescript
async function checkForUpdates() {
  const response = await fetch('/api/v1/version/check');
  const data = await response.json();
  
  if (data.update_available) {
    showUpdateNotification({
      currentVersion: data.current_version,
      latestVersion: data.latest_version,
      releaseNotes: data.release_notes,
    });
  }
}
```

---

## 📚 Additional Resources

### For UI Developers
- **API Documentation**: See [`BETA_DISTRIBUTION_GUIDE.md`](BETA_DISTRIBUTION_GUIDE.md)
- **Backend Implementation**: See [`crates/pagi-core/src/config.rs`](crates/pagi-core/src/config.rs)
- **Gateway Endpoints**: See [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs)

### Design Assets
- Phoenix logo and branding
- Color palette for beta theme
- Icon set for status indicators

---

## 🎯 Success Metrics

Track these metrics for beta users:
- Time to first successful configuration
- Configuration error rate
- Settings page usage
- API key update frequency

---

**Last Updated**: 2026-02-10  
**Version**: 0.1.0-beta.1  
**Status**: Ready for UI Implementation 🎨
