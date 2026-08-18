# Identity — The Operating Theatre

One identity, committed. This supersedes the Cortex / Synapse / Cerebra
three-way split in DESIGN_PACK.md and the ASCII wireframes transcribed from
it. Those wireframes were a sketch; they were shipped literally, emoji and
placeholder rows included, and that is what made the app read as generic.

## The idea

The tool is named Neurosurgeon. Take that seriously.

A developer's AI tooling is not a "dashboard" — it is a patient. There is one
organ (the Brain at `~/AIBrain`), thirteen grafts onto it (the tool adapters),
and a surgeon who is accountable for what they cut. `doctor` diagnoses.
`snapshot` is pre-op imaging. `rollback` reverses the operation. The
vocabulary already exists in the codebase; the surface never used it.

So the product looks and reads like a **clinical record**, not a SaaS console.

## Point of difference

Three commitments that no generic dashboard makes:

1. **Every command ends by naming the next one.** A chart is useless if it
   doesn't say what to do. No output is a dead end.
2. **Absence is reported, not hidden.** A tool that isn't installed gets a
   row saying so. Clinical records document negative findings; dashboards
   quietly show nothing.
3. **Nothing is displayed that isn't measured.** No placeholder rows, no
   sample projects, no invented counts. If a number isn't read from disk, the
   surface says so or omits it.

## Voice

Terse, factual, accountable. The register of a surgeon writing notes — plain,
specific, unhurried, no salesmanship and no apology.

- "4 of 13 tools present." — not "Great news! We found 4 tools! 🎉"
- "No Brain at ~/AIBrain." — not "Oops, something went wrong."
- "Writes 3 files. Nothing is written until you drop --dry-run."

Say what happened, what it means, and what to do next. Never exclaim. Never
thank the user for waiting. Never use "simply", "just", or "seamlessly".

## Marks

No emoji, anywhere, ever. Emoji is the single loudest tell of a generated
interface, and a clinical record has no room for it. Status is carried by a
small fixed set of glyphs, always in the first column, always one cell wide:

| Glyph | Meaning   | Use                                            |
|-------|-----------|------------------------------------------------|
| `●`   | present   | detected, healthy, in sync                     |
| `◐`   | partial   | detected but drifted, or mid-operation         |
| `▲`   | warning   | needs attention, still functional              |
| `■`   | critical  | broken, blocked, needs a human                 |
| `○`   | absent    | not installed / not found — a real finding     |
| `·`   | n/a       | not applicable to this row                     |

## Palette

Surgical, not cyberpunk. The reference is an operating theatre and the paper
chart clipped to the end of the bed: bone-white ground, drape teal, ink text,
and exactly one red reserved for genuine alarm.

| Token        | Light     | Dark      | Role                              |
|--------------|-----------|-----------|-----------------------------------|
| `ground`     | `#F4F2ED` | `#14171A` | page — bone / theatre dark        |
| `chart`      | `#FFFFFF` | `#1B1F23` | the record surface itself         |
| `rule`       | `#D9D5CC` | `#2C3238` | ruled lines; the main structure   |
| `ink`        | `#14171A` | `#E8E6E1` | primary text                      |
| `ink-soft`   | `#5C6068` | `#9AA0A6` | secondary text, units, hints      |
| `drape`      | `#0E7C6B` | `#2DBFA5` | the accent — surgical drape teal  |
| `alarm`      | `#B3261E` | `#F2685C` | critical only. Never decorative.  |
| `caution`    | `#8A6100` | `#E0A62B` | warning                           |

Red is load-bearing. If red appears anywhere that is not a critical finding,
the palette is broken.

## Structure

Charts are ruled, not boxed. **No cards, no rounded corners, no shadows, no
gradients** — those three are what make a dashboard look like every other
dashboard. Structure comes from horizontal rules and column alignment, the
way a printed form does.

- Data is monospace. A chart is a table; columns must align.
- Prose is a humanist sans.
- Density is a feature. Whitespace goes between sections, not inside rows.
- One accent per screen. If everything is teal, nothing is.

## What this rules out

Stated plainly, so it can be checked in review:

- Emoji in any surface, including commit-facing docs and CLI output.
- Placeholder or sample data rendered as if it were real.
- `bg-slate-900` cards with rounded corners and a purple primary button.
- Gradients, glows, glassmorphism, drop shadows.
- Exclamation marks in product copy.
