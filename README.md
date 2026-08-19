<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 280" width="100%" height="100%">
  <defs>
    <!-- Background Gradient -->
    <linearGradient id="bgGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0e131d" />
      <stop offset="40%" stop-color="#080b10" />
      <stop offset="100%" stop-color="#030407" />
    </linearGradient>

    <!-- Liquid Chrome Horizon Gradient (Front Faces) -->
    <linearGradient id="chromeHorizon" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#ffffff" />
      <stop offset="18%" stop-color="#dce7f5" />
      <stop offset="46%" stop-color="#8ba1b8" />
      <stop offset="49%" stop-color="#2d3b48" />
      <stop offset="51%" stop-color="#060b11" />
      <stop offset="53%" stop-color="#14212e" />
      <stop offset="74%" stop-color="#768fa8" />
      <stop offset="92%" stop-color="#dbe6f2" />
      <stop offset="100%" stop-color="#ffffff" />
    </linearGradient>

    <!-- 24K Polished Gold Gradient -->
    <linearGradient id="goldHorizon" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#ffffff" />
      <stop offset="18%" stop-color="#fff1ad" />
      <stop offset="46%" stop-color="#d49e1e" />
      <stop offset="49%" stop-color="#6e4c02" />
      <stop offset="52%" stop-color="#332200" />
      <stop offset="70%" stop-color="#c49018" />
      <stop offset="90%" stop-color="#fce277" />
      <stop offset="100%" stop-color="#fff8d4" />
    </linearGradient>

    <!-- Brushed Titanium Subtitle Bar -->
    <linearGradient id="titaniumPlate" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#1e293b" />
      <stop offset="30%" stop-color="#334155" />
      <stop offset="70%" stop-color="#1e293b" />
      <stop offset="100%" stop-color="#0f172a" />
    </linearGradient>

    <!-- Cyan Beam Gradient -->
    <linearGradient id="cyanBeam" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#00f2ff" stop-opacity="0.9" />
      <stop offset="70%" stop-color="#00f2ff" stop-opacity="0.2" />
      <stop offset="100%" stop-color="#00f2ff" stop-opacity="0" />
    </linearGradient>

    <!-- Glow & Shadow Filters -->
    <filter id="textDepthShadow" x="-10%" y="-10%" width="130%" height="140%">
      <feDropShadow dx="0" dy="8" stdDeviation="6" flood-color="#000000" flood-opacity="0.95" />
    </filter>

    <filter id="neonCyanGlow">
      <feGaussianBlur stdDeviation="2.5" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>

    <filter id="specularGlint">
      <feGaussianBlur stdDeviation="1" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
  </defs>

  <!-- Outer Card Frame -->
  <rect width="1000" height="280" rx="12" fill="url(#bgGrad)" stroke="#1e293b" stroke-width="1.5" />

  <!-- Right-Fading Neural Synaptic Weight Matrix Grid -->
  <g opacity="0.13" stroke="#38bdf8" stroke-width="1">
    <line x1="580" y1="40" x2="950" y2="40" stroke-dasharray="4 8" />
    <line x1="530" y1="90" x2="950" y2="90" />
    <line x1="500" y1="140" x2="950" y2="140" stroke-dasharray="8 4" />
    <line x1="550" y1="190" x2="950" y2="190" />
    <line x1="590" y1="240" x2="950" y2="240" stroke-dasharray="4 8" />

    <line x1="620" y1="20" x2="620" y2="260" />
    <line x1="720" y1="20" x2="720" y2="260" stroke-dasharray="6 6" />
    <line x1="820" y1="20" x2="820" y2="260" />
    <line x1="920" y1="20" x2="920" y2="260" stroke-dasharray="2 10" />
  </g>

  <!-- Synaptic Axon Trace Lines -->
  <g fill="none" opacity="0.28">
    <path d="M 450,45 L 600,45 L 660,105 L 850,105" stroke="url(#cyanBeam)" stroke-width="1.5" />
    <circle cx="660" cy="105" r="3" fill="#00f2ff" />
    <circle cx="850" cy="105" r="2" fill="#00f2ff" />
    <path d="M 520,235 L 680,235 L 740,175 L 920,175" stroke="url(#cyanBeam)" stroke-width="1.5" />
    <circle cx="740" cy="175" r="3" fill="#00f2ff" />
  </g>

  <!-- ================= LEFT-ALIGNED BRANDING ================= -->

  <!-- Top Metadata Kicker -->
  <g transform="translate(60, 48)">
    <line x1="0" y1="-4" x2="16" y2="-4" stroke="#00f2ff" stroke-width="2" filter="url(#neonCyanGlow)" />
    <text x="24" y="0" 
          font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Inter', monospace" 
          font-size="10.5" 
          font-weight="700" 
          letter-spacing="3" 
          fill="#38bdf8">
      TRANSFORMER WEIGHT SURGERY &amp; MECHANISTIC INTERPRETABILITY
    </text>
  </g>

  <!-- ================= 3D METALLIC TITLE: SYNAPSE ================= -->
  <g transform="translate(60, 138)" filter="url(#textDepthShadow)">
    
    <!-- 1. Deep 3D Extrusion Shadow Layers (Offset Depth dx, dy) -->
    <text x="6" y="6" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Montserrat', 'Arial Black', sans-serif" font-size="82" font-weight="900" letter-spacing="6" fill="#05080c">SYNAPSE</text>
    <text x="5" y="5" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Montserrat', 'Arial Black', sans-serif" font-size="82" font-weight="900" letter-spacing="6" fill="#131920">SYNAPSE</text>
    <text x="4" y="4" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Montserrat', 'Arial Black', sans-serif" font-size="82" font-weight="900" letter-spacing="6" fill="#2d2105">SYNAPSE</text>
    <text x="3" y="3" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Montserrat', 'Arial Black', sans-serif" font-size="82" font-weight="900" letter-spacing="6" fill="#594008">SYNAPSE</text>
    <text x="2" y="2" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Montserrat', 'Arial Black', sans-serif" font-size="82" font-weight="900" letter-spacing="6" fill="#997014">SYNAPSE</text>
    <text x="1" y="1" font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Montserrat', 'Arial Black', sans-serif" font-size="82" font-weight="900" letter-spacing="6" fill="#d49e1e">SYNAPSE</text>

    <!-- 2. Polished 24K Gold Bevel Rim -->
    <text x="0" y="0" 
          font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Montserrat', 'Arial Black', sans-serif" 
          font-size="82" 
          font-weight="900" 
          letter-spacing="6" 
          fill="none" 
          stroke="url(#goldHorizon)" 
          stroke-width="3" 
          stroke-linejoin="round">
      SYNAPSE
    </text>

    <!-- 3. Primary Liquid Chrome Face -->
    <text x="0" y="0" 
          font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Montserrat', 'Arial Black', sans-serif" 
          font-size="82" 
          font-weight="900" 
          letter-spacing="6" 
          fill="url(#chromeHorizon)" 
          stroke="#ffffff" 
          stroke-width="0.8">
      SYNAPSE
    </text>

    <!-- Specular Flare Glints on Letter Peaks -->
    <g filter="url(#specularGlint)">
      <!-- Flare on 'S' -->
      <g transform="translate(18, -62)">
        <polygon points="0,-9 1.5,-1.5 9,0 1.5,1.5 0,9 -1.5,1.5 -9,0 -1.5,-1.5" fill="#ffffff" />
      </g>
      <!-- Flare on 'A' -->
      <g transform="translate(262, -62)">
        <polygon points="0,-12 2,-2 12,0 2,2 0,12 -2,2 -12,0 -2,-2" fill="#ffffff" />
        <circle cx="0" cy="0" r="1.8" fill="#fff7cc" />
      </g>
      <!-- Flare on 'E' -->
      <g transform="translate(488, -62)">
        <polygon points="0,-9 1.5,-1.5 9,0 1.5,1.5 0,9 -1.5,1.5 -9,0 -1.5,-1.5" fill="#ffffff" />
      </g>
    </g>
  </g>

  <!-- ================= SUBTITLE: LLM-NEURO-SURGEON ================= -->
  <g transform="translate(60, 185)">
    <!-- Surgical Titanium Badge Plaque -->
    <polygon points="0,0 355,0 368,14 355,28 0,28" fill="url(#titaniumPlate)" stroke="#334155" stroke-width="1.2" />
    <polygon points="2,2 350,2 362,14 350,26 2,26" fill="#090d14" stroke="url(#goldHorizon)" stroke-width="0.8" />

    <!-- Left Status LED -->
    <circle cx="14" cy="14" r="3" fill="#00f2ff" filter="url(#neonCyanGlow)" />

    <!-- Subtitle Text -->
    <text x="28" y="19" 
          font-family="-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Montserrat', 'Arial Black', sans-serif" 
          font-size="12" 
          font-weight="800" 
          letter-spacing="5" 
          fill="url(#goldHorizon)">
      LLM-NEURO-SURGEON
    </text>

    <!-- Version / Active Tag -->
    <g transform="translate(390, 0)">
      <rect x="0" y="3" width="70" height="22" rx="4" fill="#0f172a" stroke="#1e293b" stroke-width="1" />
      <text x="35" y="17" font-family="monospace" font-size="10" font-weight="700" fill="#38bdf8" text-anchor="middle">v1.0.0</text>
    </g>
  </g>

  <!-- ================= BOTTOM TECHNICAL PILLS ================= -->
  <g transform="translate(60, 240)">
    <!-- Pill 1 -->
    <g transform="translate(0, 0)">
      <rect width="128" height="18" rx="9" fill="#0b111a" stroke="#1e293b" stroke-width="0.8" />
      <circle cx="9" cy="9" r="2" fill="#00f2ff" />
      <text x="68" y="12.5" font-family="monospace" font-size="8" font-weight="700" letter-spacing="1" fill="#94a3b8" text-anchor="middle">DIFF-MASKING</text>
    </g>

    <!-- Pill 2 -->
    <g transform="translate(136, 0)">
      <rect width="160" height="18" rx="9" fill="#0b111a" stroke="#1e293b" stroke-width="0.8" />

## 💡 What is Synapse (LLM-NeuroSurgeon)?

**Synapse (LLM-NeuroSurgeon)** is the local-first configuration engine and synchronizer that keeps Claude Code, Cursor, Gemini CLI, Windsurf, Zed, and 8+ other AI coding companions in permanent lockstep.

```
                     ┌────────────────────────┐
                     │       ~/AIBrain        │
                     │  (Single Source Truth) │
                     └───────────┬────────────┘
                                 │
        ┌──────────────┬─────────┴────────┬──────────────┐
        ▼              ▼                  ▼              ▼
   Claude Code       Cursor           Gemini CLI      Windsurf / Zed
  (`.claude/`)   (`.cursorrules`)    (`GEMINI.md`)    (`.windsurfrules`)
```

---

## ⚡ 60-Second Quickstart

```bash
# 1. Detect active AI coding tools on your machine
cargo run -p neurosurgeon -- scan

# 2. Ingest configurations into ~/AIBrain (Git-backed repository)
cargo run -p neurosurgeon -- import --dry-run
cargo run -p neurosurgeon -- import

# 3. Launch background auto-sync daemon with 3-way merge resolution
cargo run -p neurosurgeon -- sync --daemon
```

For full setup prerequisites across Linux, macOS, and Windows, read the **[Quickstart Guide](docs/QUICKSTART.md)**.

---

## 📚 Documentation Index

| Guide | Description | Target |
|---|---|---|
| **[Docs Hub](docs/README.md)** | Centralized documentation navigation & command reference | All users & contributors |
| **[Quickstart](docs/QUICKSTART.md)** | Step-by-step setup in under 60 seconds | First-time setup |
| **[User Guide](docs/USER_GUIDE.md)** | Day-to-day workflow, daemon sync, MCP hub & Doctor self-healing | Daily development |
| **[Architecture](docs/ARCHITECTURE.md)** | 3-way merge engine, file system watcher & monorepo layout | Engine internals |
| **[Adapters Hub](docs/adapters/README.md)** | Complete matrix and individual adapter specifications | Tool dialect reference |
| **[Contributing](docs/development/CONTRIBUTING.md)** | PR lifecycle, test requirements & coding standards | Open source contributors |

---

## 🩺 The Doctor: Self-Healing Configurations

When tool configurations drift or symlinks break, Synapse detects and repairs the issue automatically:

```bash
cargo run -p neurosurgeon -- doctor
cargo run -p neurosurgeon -- doctor --fix
```

---

<div align="center">
<sub>Built with Rust, Tauri 2, and React. Open source under the MIT License.</sub>
</div>
