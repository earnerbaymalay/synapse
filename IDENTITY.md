# Identity — SYNAPSE

One identity, committed. This supersedes both the Cortex / Synapse / Cerebra
three-way split in DESIGN_PACK.md and this file's own prior draft ("The
Operating Theatre"). That draft's structural rules turned out to be right —
they're kept below — but its ad-hoc palette and naming are now replaced by
the real, fully-specified brand system in `brands/synapse/`.

**Canonical references:**
- [`brands/synapse/marketing-pack/BRAND_BOOK.md`](brands/synapse/marketing-pack/BRAND_BOOK.md) — voice, palette, type, structural system
- [`brands/synapse/tokens.json`](brands/synapse/tokens.json) — machine-readable tokens
- [`brands/synapse/index.html`](brands/synapse/index.html) — visual reference, open in a browser

## The idea

The tool is named Neurosurgeon; the product is SYNAPSE. Take the metaphor
seriously. A developer's AI tooling is not a "dashboard" — it is a patient.
One organ (the Brain at `~/AIBrain`), thirteen grafts onto it (the tool
adapters), a surgeon accountable for what they cut. `doctor` diagnoses.
`snapshot` is pre-op imaging. `rollback` reverses the operation.

So the product reads like a **clinical record**, not a SaaS console — now
rendered in SYNAPSE's actual palette (ink/accent-blue/gold-rare) instead of
a placeholder one.

## What still holds, unconditionally

These are the rules that made the CLI and desktop app stop reading as
generated. They are brand-agnostic and apply regardless of which palette is
current:

1. **Every command ends by naming the next one.** No output is a dead end.
2. **Absence is reported, not hidden.** A tool that isn't installed gets a
   row saying so.
3. **Nothing is displayed that isn't measured.** No placeholder rows, no
   sample projects, no invented counts.
4. **No emoji, anywhere, ever.** Status is carried by a small fixed glyph
   set (`● ◐ ▲ ■ ○ ·`), one cell wide, first column.
5. **No cards, no rounded corners, no shadows, no gradients.** Structure
   comes from ruled lines and column alignment — SYNAPSE's own version of
   this is the blueprint corner-mark system in the brand book, not a return
   to soft UI.

## What changed from the prior draft

- Palette: SYNAPSE's ink/accent-blue/gold-rare/semantic-green, not the
  bone/teal "Operating Theatre" palette. Dark-only — the brand system has no
  light variant, so the app no longer follows `prefers-color-scheme`.
- Status color is no longer the same as the UI accent: "present/healthy"
  uses semantic green, the accent blue is reserved for links, buttons, and
  the wordmark — see `BRAND_BOOK.md` Do/Don't.
- Typography: three fonts (Rubik Mono One for the wordmark, Inter for UI,
  JetBrains Mono for data), vendored locally under
  `apps/desktop/src/assets/fonts/` rather than fetched from a CDN — the
  brand's own claim is "local-first, zero telemetry," so a font fetch on
  every launch would contradict the product it's branding.
- CLI binary renamed `neurosurgeon` → `synapse` (the internal
  `neurosurgeon-core` library crate and `NEUROSURGEON_*` env vars are
  unchanged — that's implementation detail, not brand surface).

## What this still rules out

- Emoji in any surface, including commit-facing docs and CLI output.
- Placeholder or sample data rendered as if it were real.
- Gold used for anything but the marketing hero device — never a button,
  link, or status color (brand book rule, now enforced: the app's Tailwind
  config has no gold token at all).
- Rounded corners, drop shadows, glassmorphism, gradients on UI chrome.
- Exclamation marks in product copy.
