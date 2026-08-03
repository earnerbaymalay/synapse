# LLM Neurosurgeon — Aesthetics & Branding Overhaul

## Context
The user wants to update the repository's aesthetics, design, documentation, and onboarding flow, and generate a comprehensive marketing/branding pack. 
- **Selected Brand:** `Synapse` (Developer-centric, dark-mode, monospace).
- **Marketing Tone:** `The Minimalist Approach` ("Effortless AI Configuration. One Brain, All Models").
- **Goal:** Move from a "technical scaffold" feel to a "polished product" feel.

## Solution Design

### 1. Aesthetics & Design Update
The `Synapse` identity will be elevated from a simple HTML mockup to a full design system.
- **Primary Palette:** Deep space black (#0d1117), Synapse Blue (#58a6ff), and muted Slate (#8b949e).
- **Typography:** Monospace-first (JetBrains Mono, SF Mono) for a "coding-native" feel.
- **UI Logic:** Implement a "Glassmorphism" layer over dark backgrounds with high-contrast accents for active states.

### 2. Documentation & User Onboarding
The current `USER_GUIDE.md` is comprehensive but technical. We will transform it into a conversion-focused "getting started" experience.
- **Onboarding Flow:** 
    - *Phase 1: The Hook* (The "fragmentation" pain point).
    - *Phase 2: The Solution* (The "Brain" concept).
    - *Phase 3: Immediate Value* (The `scan` $\to$ `import --dry-run` $\to$ `project` loop).
    - *Phase 4: Long-term Power* (Time Machine, MCP Hub, Doctor).
- **Visuals:** Integrate ASCII-art flowcharts and better Markdown structure (Callouts, Tables) to reduce cognitive load.

### 3. Marketing & Branding Pack
A new directory `brands/synapse/marketing-pack/` will be created containing:
- **Brand Book (`BRAND_BOOK.md`):** Color codes, typography rules, and voice guidelines.
- **Value Proposition Matrix:** Feature $\to$ Benefit mapping for different user personas (Solo Dev, Team Lead, AI Researcher).
- **Asset Templates:** 
    - `social_preview.md`: Copy and layout for Twitter/LinkedIn.
    - `landing_page_v2.html`: A modern, minimalist, single-page landing site based on the Synapse aesthetic.
    - `taglines.txt`: A list of punchy one-liners for various uses.

## Risks
- **Over-design:** Too much "fluff" might alienate the technical target audience. We will stick to the "Minimalist Approach."
- **Consistency:** Ensuring the `README.md` and `USER_GUIDE.md` align with the new brand voice.

## Non-goals
- No actual image generation (AI image prompts provided instead).
- No changes to the Rust/Tauri core logic (this is a purely aesthetic/content layer update).

## Low-Level Design

### File Changes
- **`README.md`**: Rewrite intro, value prop, and quickstart to match the "Minimalist" voice.
- **`docs/USER_GUIDE.md`**: Restructure into a guided "Onboarding Journey" rather than a manual.
- **`brands/synapse/`**: 
    - Create `marketing-pack/` folder.
    - Update `index.html` to be the "Core Identity" reference.
    - Create `marketing-pack/BRAND_BOOK.md`.
    - Create `marketing-pack/landing_page_v2.html`.
    - Create `marketing-pack/taglines.txt`.
    - Create `marketing-pack/social_assets.md`.

### Content Strategy
- **Tone:** "Surgical precision. Zero friction."
- **Key Phrase:** "One Brain. All Models."

## Tasks summary
1. Update `README.md` for a high-conversion, minimalist "product" feel.
2. Rewrite `docs/USER_GUIDE.md` as a guided onboarding journey.
3. Build the `brands/synapse/marketing-pack/` directory and its assets.
4. Update the Synapse reference dashboard for a a more polished look.
5. Final verification and commit.


## Implementation Log — 2026-08-01 09:54
**Summary:** Aesthetics & branding overhaul: README productized, USER_GUIDE restructured as onboarding journey, marketing pack created (BRAND_BOOK, landing page, taglines, social assets), Synapse dashboard polished with glassmorphism design system.
**Changed files:** A	.claudinio/plans/2026-08-01_aesthetics-branding-update.md, M	Cargo.toml, M	PROGRESS.md, M	README.md, M	apps/desktop/src-tauri/Cargo.toml, M	brands/synapse/index.html, A	brands/synapse/marketing-pack/BRAND_BOOK.md, A	brands/synapse/marketing-pack/landing_page_v2.html, A	brands/synapse/marketing-pack/social_assets.md, A	brands/synapse/marketing-pack/taglines.txt, M	docs/USER_GUIDE.md, M	packages/core/src/adapters/mod.rs, M	packages/core/src/adapters/opencode.rs, M	packages/core/src/mcp_registry.rs
**Commits:** 6947b44 Aesthetics & branding overhaul: Synapse identity, 4d82bc2 docs(plan): aesthetics-branding-update, d9dd0b4 docs: update PROGRESS.md with 2026-08-01 resume session entry, d2b5063 chore(core): clippy lint fixes + deduplicate release profile
**Journal:** ## Key Decisions & Findings

**README.md Productization**: The original README was thorough but read like a technical spec. The rewrite reframes it as a product landing page — leading with "One Brain. All Models." as the hero value prop, using "Surgical precision. Zero friction." as the tone anchor, and restructuring the problem/solution narrative around a relatable developer story (2 AM config drift). All technical content (badges, 12-tool list, architecture diagram, license) was preserved — only the framing changed.

**USER_GUIDE → Onboarding Journey**: The biggest structural change. The original was a flat reference manual (12 sections, all equal weight). The rewrite organizes it into 4 phases that mirror the user's emotional journey: pain (Hook) → discovery (Solution) → quick win (Immediate Value) → mastery (Power User). This makes the guide scannable and conversion-focused. Key technique: the "3-Command Onboarding Loop" (scan → import --dry-run → import) gives users a dopamine hit in Phase 3 before they ever reach the advanced features.

**Marketing Pack**: The BRAND_BOOK.md is the single source of truth for the Synapse identity — all other assets reference its palette and voice. The landing_page_v2.html is a standalone HTML file (no dependencies) that could be deployed as-is. The taglines.txt and social_assets.md give the user ready-to-use copy for any channel.

**Dashboard Polish**: The original index.html had a "SYNAPS" typo and basic styling. The rewrite adds glassmorphism, a grid pattern overlay, a terminal-style status bar with pulsing dot, and quick-action buttons — making it a proper brand reference card. The CSS custom properties ensure it stays consistent with the Brand Book.

**Consistency**: All files now share the same color palette (#0d1117, #58a6ff, #8b949e), monospace font stack, and voice. The brand identity is coherent across documentation, marketing assets, and the reference dashboard.

**Task journal:**
- README.md Productization: Hero section now reads 'One Brain. All Models.' with 'Effortless AI Configuration.' tagline; Problem section rewritten as relatable developer narrative (2 AM story); Solution reframed as 'Surgical precision. Zero friction.' with benefit-oriented 4-step flow; Quickstart renamed to '30-Second Quickstart' with tighter command comments; All badges, CI links, 12-tool list, architecture diagram, and license preserved
- User Guide → Onboarding Journey: Restructured from flat technical sections into 4-phase onboarding journey; Phase 1: The Hook — opens with relatable 2 AM developer story + [!WARNING] callout; Phase 2: The Solution — Brain concept with [!TIP] 'One Brain. All Models.' callout; Phase 3: Immediate Value — new Quick Start section with '3-Command Onboarding Loop'; Phase 4: Long-term Power — Time Machine, MCP Hub, Doctor, Marketplace, FAQ; All technical content, code blocks, directory trees, and 12-tool matrix preserved intact
- Generate Marketing Pack Assets: Created brands/synapse/marketing-pack/ directory; BRAND_BOOK.md — 57 lines: color palette, typography, voice guidelines, logo usage, do's/don'ts; landing_page_v2.html — 537 lines: full single-page landing site with glassmorphism, hero, problem, solution, features, footer; taglines.txt — 47 lines: 25 taglines across 6 categories (core, product, dev, social, feature, competitive); social_assets.md — 88 lines: Twitter/X and LinkedIn copy templates, hashtags, visual guidelines
- Polishing Synapse Reference Dashboard: Added CSS custom properties for full Synapse color palette; Added subtle grid pattern overlay on body background; Refined animated gradient bar with shimmer animation (4s cycle); Glassmorphism stat cards with backdrop-filter: blur(12px) and hover lift effect; New terminal status bar with green pulsing dot and blinking cursor; Quick actions row: Scan Environment (primary), Run Doctor (outline), View Dashboard (outline); Responsive: single-column layout at 600px breakpoint; Fixed 'SYNAPS' typo to 'SYNAPSE'
- Consistency Review & Final Commit: Consistency check passed: all files use 'One Brain. All Models.' tagline, 'Surgical precision. Zero friction.' tone, Synapse Blue (#58a6ff) accent, monospace typography; README and USER_GUIDE both use the Minimalist voice with callouts and benefit-oriented framing; Brand Book defines the exact palette used across all assets; landing_page_v2.html and index.html share the same glassmorphism design language; Committed as 6947b44 with 7 files changed, 1197 insertions, 238 deletions


## Implementation Log — 2026-08-01 11:26
**Summary:** Phase 5 GUI tests fixed (jsdom downgrade for Node 18) — all 4 e2e tests pass, all 8 screens build. T8.3 Gate 4 ready: CLI binary, desktop dist, packaging docs, reproducible-build notes all verified.
**Changed files:** A	.claudinio/plans/2026-08-01_aesthetics-branding-update.md, M	Cargo.toml, M	PROGRESS.md, M	README.md, M	apps/desktop/package.json, M	apps/desktop/src-tauri/Cargo.toml, M	brands/synapse/index.html, A	brands/synapse/marketing-pack/BRAND_BOOK.md, A	brands/synapse/marketing-pack/landing_page_v2.html, A	brands/synapse/marketing-pack/social_assets.md, A	brands/synapse/marketing-pack/taglines.txt, M	docs/USER_GUIDE.md, M	packages/core/src/adapters/mod.rs, M	packages/core/src/adapters/opencode.rs, M	packages/core/src/mcp_registry.rs, M	pnpm-lock.yaml
**Commits:** fad6c6d fix(phase5): downgrade jsdom 29→24 for Node 18 compatibility, 6947b44 Aesthetics & branding overhaul: Synapse identity, 4d82bc2 docs(plan): aesthetics-branding-update, d9dd0b4 docs: update PROGRESS.md with 2026-08-01 resume session entry, d2b5063 chore(core): clippy lint fixes + deduplicate release profile
**Journal:** ## Phase 5 GUI — Fix & Verify

The vitest test runner was broken by a jsdom version mismatch: jsdom@29 requires Node >= 22, but this environment has Node 18.19.1. Downgraded jsdom to ^24.1.0 (the latest compatible with Node 18). All 4 OnboardingWizard e2e tests now pass: full 3-step dry-run flow, empty state, graceful degradation when no Tauri backend is present, and back-navigation. All 8 GUI screens (MainDashboard, ConfigurationManager, AdapterInspector, StatusMonitor, DebugConsole, OnboardingWizard, Marketplace, McpHub) build cleanly via tsc + vite build.

## T8.3 Gate 4 — Ready for Human Install

Everything is prepared for the user to install on a real machine. The CLI binary (1.1 MB) is built and functional. The desktop frontend dist is built. Packaging docs cover all 4 platform installers (.dmg, .msi, .deb, .AppImage), Homebrew formula, and shell installer scripts. The reproducible-builds doc specifies deterministic toolchains and SHA-256 manifests.

**Task journal:**
- T8.3 Gate 4 — Prepare for human install & onboarding: CLI binary: 1.1 MB release binary at target/release/neurosurgeon; Desktop frontend: dist/ built (0.40 KB HTML + 11.29 KB CSS + 181.56 KB JS); CLI scan: detects claude-code in this repo; CLI import --dry-run: reports 7 skills, 6 agents, 0 MCP servers — nothing written; CLI doctor --fix: initializes git repo + mappings.json in ~/AIBrain; Packaging docs: docs/packaging/RELEASE_PACKAGING.md with .dmg/.msi/.deb/.AppImage configs, Homebrew formula, install.sh/install.ps1; Reproducible builds: docs/REPRODUCIBLE_BUILDS.md with deterministic toolchain, dependency locking, SHA-256 manifests; All Rust tests pass, all desktop tests pass (4/4), frontend builds clean
- Phase 5 GUI — Fix tests & verify all 8 screens: jsdom downgraded from ^29.1.1 to ^24.1.0 (Node 18 compatibility); All 4 OnboardingWizard e2e tests pass: full 3-step flow, empty state, graceful degradation, back navigation; All 8 screens build successfully (tsc + vite build); Frontend dist: 0.40 KB HTML + 11.29 KB CSS + 181.56 KB JS
