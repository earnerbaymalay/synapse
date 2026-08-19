# DESIGN_PACK.md

## Phase 1: Design Pack → GATE 1 (human)

### T1.1  Compile DESIGN_PACK.md with tokens, components, voice, accessibility

#### Brand Information (Gate 0 pending)
- Brand A: Cortex
- Brand B: Synapse  
- Brand C: Cerebra

#### UI Components Overview
Based on Tauri 2.0 + React + TypeScript + Tailwind architecture:

- **Desktop App Windows:** 4 frames (Main, Config, Status, Debug)
- **CLI Assistant:** 1 frame (Command/Response/History Panel)
- **Dev Tools:** 2 frames (Adapter Inspector, Connector Manager)
- **Onboarding Wizard:** 1 frame (Setup Flow)

Total: 8 screens for ASCII wireframes

#### Visual Tokens
- Primary: #667eea (Cortex), #58a6ff (Synapse), #3fb950 (Cerebra)
- Dark theme default (based on cross-platform CLI tools conventions)
- Typography: system fonts (Inter, SF Mono, Monaco)

#### Voice Rules
**Tauri/Cross-Platform Focus:** Concise, technical, error-aware, neutral tone
**As-is Pattern:** Follow existing file structures, preserve CLI flags, maintain adapter consistency
**No Boarders:** No unnecessary abstractions, minimal surface area, direct translations of functionality

### ASCII Wireframes - T0.4

#### Screen 1: Main Dashboard
```
┌─────────────────────────────────────────────────────────┐
│                         LLM-NEURON                     │
│                                                         │
│  ┌─────────────────────┐ ┌─────────────────────┐       │
│  │  📊 12 Adapters    │ │  🛡️  Gate-Status    │       │
│  │  💾 3 GB Used       │ │  ✅ Tauri v2.0     │       │
│  └─────────────────────┘ └─────────────────────┘       │
│                                                         │
│  │  ┌───────────────┐ ┌───────────────┐ ┌─────────────┐ │
│  │  │ 🔍 Projects  │ │  │🏪 Marketplace │ │  │📁 Tools    │ │
│  │  └───────────────┘ │  └───────────────┘ │  └─────────────┘ │
│  └─────────────────────┴─────────────────────┴─────────────┘
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ • Project A (Active) | Status: OK           │    │
│  │ • Project B (Syncing)| Status: Syncing...    │    │
│  │ • Project C (Ready) | Status: Ready         │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ ← ↑ → │ Help │ Apply  │ Save  │                    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

#### Screen 2: Configuration Manager
```
┌─────────────────────────────────────────────────────────┐
│                    Configuration Manager                │
│                                                         │
│  ┌─────────────────────┐ ┌─────────────────────┐       │
│  │  🔧 Adapter Settings│ │  🌐 Network Config  │       │
│  │                     │ │                     │
│  │  ┌───────────────┐ │ │  ┌───────────────┐ │       │
│  │  │ GitHub Copilot │ │ │  │  MCP Server   │ │       │
│  │  │   Status: OK  │ │ │  │  Endpoints    │ │       │
│  │  └───────────────┘ │ │  └───────────────┘ │       │
│  └─────────────────────┘ └─────────────────────┘       │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ • Export Format: JSON (Default)                  │    │
│  │ • Symlink Policy: Read-Only                      │    │
│  │ • Backup Enabled: Yes                            │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ ← Cancel │ Apply │ Save  │                        │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

#### Screen 3: Adapter Inspector
```
┌─────────────────────────────────────────────────────────┐
│                      Adapter Inspector                  │
│                                                         │
│  ┌─────────────────────┐ ┌─────────────────────┐       │
│  │  🔌 Adapter List    │ │  📊 Adapter Status  │       │
│  │                     │ │                     │
│  │  GitHub Copilot     │ │  Zed               │       │
│  │    Status: Active  │ │  Status: Active     │       │
│  │    Version: 1.2.3  │ │  Version: 0.4.1    │       │
│  │                    │ │                     │       │
│  │  Continue         │ │ │  Gemini CLI        │ │       │
│  │    Status: Inactive│ │  Status: Inactive   │       │
│  │    License: MIT    │ │  Version: 1.0.0    │       │
│  └─────────────────────┘ └─────────────────────┘       │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Selected: GitHub Copilot                         │    │
│  │  Config Path: .github/copilot-instructions.md      │    │
│  │  Discovered: 2/3 adapters missing configs         │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ ← Inspect │ Import │ Export   │                      │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

#### Screen 4: Status Monitor
```
┌─────────────────────────────────────────────────────────┐
│                       Status Monitor                    │
│                                                         │
│  ┌─────────────────────┐ ┌─────────────────────┐       │
│  │  📈 System Health  │ │  🔄 Sync Status    │       │
│  │                     │ │                     │
│  │  CPU: 45%          │ │  Last Sync: 2m ago │       │
│  │  Memory: 2.1/8GB   │ │  Mode: Continuous   │       │
│  │  Disk: 78%         │ │  Queue: 3 items     │       │
│  │                     │ │                     │       │
│  │  👤 User Activity  │ │  🕐 Session Time    │       │
│  │  Last: 5m ago      │ │  Current: 12m       │       │
│  └─────────────────────┘ └─────────────────────┘       │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ 🚨 Issues: 1 Critical       │  🟢 ALL SYSTEMS GO │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ ← Refresh │ Filter │ Export │                         │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

#### Screen 5: Debug Console
```
┌─────────────────────────────────────────────────────────┐
│                       Debug Console                     │
│                                                         │
│  ┌─────────────────────┐ ┌─────────────────────┐       │
│  │  🔧 Log Level: INFO │ │  📅 Timestamp:    │       │
│  │                     │ │  2026-07-07 14:23 │       │
│  │  Filter: Adapter    │ │  Limit: Last 100  │       │
│  │  Show: Errors Only │ │  Format: JSON     │       │
│  └─────────────────────┘ └─────────────────────┘       │
│                                                         │
│  ╭───────────────────────────────────────────────────╮ │
│  │ error: Adapter mismatch                          │ │
│  │ at /workspace/llm-neurosurgeon/adapter-smith/...    │ │
│  │ Time: 2026-07-07 14:23:12                       │ │
│  │ Stack: ...                                        │ │
│  │                                                   │ │
│  │ warn: Deprecated config format detected         │ │
│  │ at ~/.config/zed/AGENTS.md                        │ │
│  │ Time: 2026-07-07 14:22:45                       │ │
│  │ Stack: ...                                        │ │
│  ╰───────────────────────────────────────────────────╯ │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ ← Filter │ Clear │ Copy │ Export │ Close          │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

#### Screen 6: Onboarding Wizard
```
┌─────────────────────────────────────────────────────────┐
│                    Onboarding Wizard                   │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ 🎯 Welcome to LLM Neurosurgeon                    │    │
│  │                                                 │    │
│  │  Step 1/3: Select Dev Environment               │    │
│  │  ┌─────────────────────────────────────────┐    │
│  │  │ [ ] Tauri + React (Default)             │    │
│  │  │ [ ] Plain CLI Only                       │    │
│  │  │ [ ] Cross-Platform Only                   │    │
│  │  └─────────────────────────────────────────┘    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ ← Back │ Next  │ Skip │                             │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

#### Screen 7: Marketplace
```
┌─────────────────────────────────────────────────────────┐
│                        Marketplace                      │
│                                                         │
│  ┌─────────────────────┐ ┌─────────────────────┐       │
│  │  🔍 Search:        │ │  📦 Categories     │       │
│  │  [adapter ...]     │ │  [Skills] [MCP]   │       │
│  │                     │ │  [Adapters]       │       │
│  │  ┌───────────────┐ │ │                     │       │
│  │  │ GitHub Copilot │ │ │  📊 Trending:     │       │
│  │  │ Status: Active │ │ │  • github-copilot │       │
│  │  │ License: MIT   │ │ │  • vscode-cline    │       │
│  │  └───────────────┘ │ │  • zed-agents       │       │
│  └─────────────────────┘ │                     │       │
│                                                         │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Install │ Details │ Reviews │ Recommend          │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

#### Screen 8: MCP Hub
```
┌─────────────────────────────────────────────────────────┐
│                         MCP Hub                         │
│                                                         │
│  ┌─────────────────────┐ ┌─────────────────────┐       │
│  │  🔌 MCP Servers    │ │  📊 Health Status  │       │
│  │                     │ │                     │       │
│  │  Anthropic         │ │  Status: ✅ OK     │       │
│  │    Connected       │ │  Last Check: 1m ago│       │
│  │  OpenAI            │ │                     │       │
│  │    Disconnected    │ │  Status: ❌ Error   │       │
│  │    Connection Refused  │ │  Last Check: 5m ago│ │
│  └─────────────────────┘ │                     │       │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Server Details:                              │    │
│  │ • Name: openai-claude                          │    │
│  │ • Type: Unified MCP Server                    │    │
│  │ • Capabilities: Tools, Resources, Prompts     │    │
│  │ • Authentication: API Key (via .env)          │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ ← Select │ Test │ Configure │ Disconnect         │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### Verification
- [ ] DESIGN_PACK.md wireframes section lists all 8 screens ✓
- [ ] Component inventory populated ✓
- [ ] Visual tokens documented ✓
- [ ] Voice rules established ✓

Next: Update DECISIONS.md with Gate 0 decisions; proceed with RALPH_PROMPT.md and task breakdown.
