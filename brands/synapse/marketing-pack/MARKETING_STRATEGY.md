# SYNAPSE — Marketing Strategy

## Positioning

**For** developers running more than one AI coding tool, **SYNAPSE is** the canonical configuration layer **that** unifies every tool's rules, skills and MCP servers into one git-backed Brain, **unlike** manually copy-pasting between `CLAUDE.md`, `.cursorrules` and friends, or locking into a single vendor's tool.

**Category:** developer infrastructure / AI tooling, not "another AI app." SYNAPSE never talks to a model — it manages the config layer underneath all of them. That distinction is the whole pitch.

## Personas

1. **The Polyglot Solo Dev** — runs 3-5 AI tools daily, feels the copy-paste tax personally. Primary audience. Wins on the 3-command loop's immediate payoff.
2. **The Team Lead** — wants every engineer's AI tooling configured identically. Wins on git-backed history + one Brain shared via a repo.
3. **The AI Power User / Researcher** — lives in MCP servers and custom skills, wants a marketplace and provenance tracking. Wins on the Marketplace + SHA-256 verification story.

## Messaging pillars

1. **One Brain, not another tool.** SYNAPSE has no model of its own — it's infrastructure. Say this early and often; it's the single biggest objection to preempt ("do I need to switch tools?" — no).
2. **Surgical, not soft.** Dry-run by default, Git snapshot before anything destructive, zero telemetry. Precision is the brand, not a footnote.
3. **The config layer is version-controlled now.** Most devs have never thought of their AI rules as something with a diff and a rollback. That reframe is the "aha."

## Competitive framing

- **vs. doing nothing (manual copy-paste):** the honest default competitor. Win on time saved and drift eliminated.
- **vs. a single-vendor tool (e.g. staying inside one IDE's ecosystem):** SYNAPSE is vendor-neutral by design — it gets stronger as you add more tools, not weaker.
- **Never punch down at individual adapters' tools.** They're the audience, not the enemy — SYNAPSE makes all of them better together.

## Channel plan

| Channel | Angle | Asset |
|---|---|---|
| Hacker News (Show HN) | Technical depth, real numbers | `SYNAPSE_Launch_Copy.md` → HN section |
| Product Hunt | Visual + tagline, launch-day push | Social templates, PH description |
| Reddit — r/programming, r/rust, r/LocalLLaMA | Tailored per community norm | `SYNAPSE_Launch_Copy.md` → Reddit section |
| X / Twitter | Thread + terminal GIF | Launch thread, asciinema recording |
| dev.to / Hashnode | Long-form "why we built this" | Article outline in Launch Copy |
| GitHub itself | README is the real landing page | `SYNAPSE_README.md`, social preview image |

## Launch checklist

**Pre-launch**
- [ ] README replaced with `SYNAPSE_README.md` content
- [ ] GitHub social preview image set (Settings → Social preview) using the 1280×640 template
- [ ] `docs/ONBOARDING.md` added, linked from README
- [ ] Record a real terminal session with `asciinema` or `vhs` for the scan → import → project loop
- [ ] Tag a `v1.0.0` release with real changelog

**Launch day**
- [ ] Show HN post (morning, US time zones)
- [ ] Product Hunt submission scheduled for 12:01 AM PT
- [ ] X thread + one Reddit post, spaced a few hours apart — never all channels at once

**Post-launch (week 1)**
- [ ] Respond to every HN/PH/Reddit comment within a few hours
- [ ] Ship one visible fix or feature from launch feedback — momentum signal
- [ ] dev.to article once the dust settles, linking back to the repo

## Success metrics

GitHub stars are vanity; watch **adapter diversity in issues/discussions** (are people using it with tools beyond Claude/Cursor?) and **return usage signals** (doctor/sync mentioned in later posts) as the real signal the "Brain" concept stuck.
