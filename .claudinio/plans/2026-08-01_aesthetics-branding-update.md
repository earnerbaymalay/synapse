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
