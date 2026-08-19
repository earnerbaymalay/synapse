//! The clinical record renderer — see IDENTITY.md ("The Operating Theatre").
//!
//! Every surface the CLI prints is a chart: a ruled header, aligned columns
//! with a one-cell status glyph in the first position, and a footer that
//! names the next command. There are no boxes and no emoji.
//!
//! Colour is applied only when stdout is a terminal and `NO_COLOR` is unset,
//! so piped and redirected output stays plain text. Deliberately dependency
//! -free: the palette is small enough that a crate would cost more than it
//! saves.

use std::io::IsTerminal;
use std::sync::OnceLock;

/// Width of the ruled lines. Narrow enough for a split terminal, wide enough
/// for a path plus three count columns.
pub const WIDTH: usize = 74;

// ── colour ──────────────────────────────────────────────────────────────

/// True when stdout is an interactive terminal and the user has not opted
/// out via `NO_COLOR` (https://no-color.org) or `TERM=dumb`.
fn color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        if matches!(std::env::var("TERM").as_deref(), Ok("dumb")) {
            return false;
        }
        std::io::stdout().is_terminal()
    })
}

/// The SYNAPSE palette (brands/synapse/tokens.json), mapped to the 256
/// -colour cube so it survives terminals without truecolour support.
#[derive(Clone, Copy)]
pub enum Paint {
    /// Secondary text: units, hints, absent rows.
    InkSoft,
    /// The one working UI accent — Synapse Blue. Links, the wordmark, the
    /// next-command hint. Never status: see `Success`.
    Drape,
    /// Healthy / present. Deliberately not the same colour as `Drape` — the
    /// brand book scopes accent to links/buttons/focus, status gets its own
    /// semantic green.
    Success,
    /// Critical only. Never decorative.
    Alarm,
    /// Warning.
    Caution,
    /// Structural rules and column separators.
    Rule,
    /// Emphasis within otherwise plain text.
    Bold,
}

impl Paint {
    fn code(self) -> &'static str {
        match self {
            Paint::InkSoft => "\x1b[38;5;245m",
            Paint::Drape => "\x1b[38;5;33m",
            Paint::Success => "\x1b[38;5;77m",
            Paint::Alarm => "\x1b[38;5;167m",
            Paint::Caution => "\x1b[38;5;179m",
            Paint::Rule => "\x1b[38;5;240m",
            Paint::Bold => "\x1b[1m",
        }
    }
}

/// Wraps `text` in `paint`, or returns it untouched when colour is disabled.
pub fn paint(paint: Paint, text: &str) -> String {
    if color_enabled() {
        format!("{}{}\x1b[0m", paint.code(), text)
    } else {
        text.to_string()
    }
}

// ── status glyphs ───────────────────────────────────────────────────────

/// The fixed status vocabulary from IDENTITY.md. Always one cell wide, always
/// the first column of a row.
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // the full vocabulary is the spec — see IDENTITY.md
pub enum Mark {
    /// Detected, healthy, in sync.
    Present,
    /// Detected but drifted, or mid-operation.
    Partial,
    /// Needs attention, still functional.
    Warning,
    /// Broken or blocked — needs a human.
    Critical,
    /// Not installed or not found. A real finding, not an empty cell.
    Absent,
    /// Not applicable to this row.
    NotApplicable,
}

impl Mark {
    /// The bare glyph, without colour. One character wide in every case.
    pub fn glyph(self) -> char {
        match self {
            Mark::Present => '●',
            Mark::Partial => '◐',
            Mark::Warning => '▲',
            Mark::Critical => '■',
            Mark::Absent => '○',
            Mark::NotApplicable => '·',
        }
    }

    fn tint(self) -> Paint {
        match self {
            Mark::Present => Paint::Success,
            Mark::Partial | Mark::Warning => Paint::Caution,
            Mark::Critical => Paint::Alarm,
            Mark::Absent | Mark::NotApplicable => Paint::InkSoft,
        }
    }

    /// The glyph, tinted for the terminal.
    pub fn render(self) -> String {
        paint(self.tint(), &self.glyph().to_string())
    }
}

// ── chart furniture ─────────────────────────────────────────────────────

/// Opens a chart: the procedure name, right-aligned context, and a rule.
///
/// ```text
///   SYNAPSE · INTAKE                               4 of 13 present
///   ──────────────────────────────────────────────────────────────
/// ```
pub fn open(procedure: &str, context: &str) {
    let title = format!("SYNAPSE · {}", procedure.to_uppercase());
    let pad = WIDTH.saturating_sub(title.chars().count() + context.chars().count());
    println!();
    println!(
        "  {}{}{}",
        paint(Paint::Bold, &title),
        " ".repeat(pad.max(1)),
        paint(Paint::InkSoft, context),
    );
    rule();
}

/// A full-width horizontal rule — the chart's only structural element.
pub fn rule() {
    println!("  {}", paint(Paint::Rule, &"─".repeat(WIDTH)));
}

/// A labelled scalar above the table, e.g. `Site   /home/user/project`.
pub fn field(label: &str, value: &str) {
    println!("  {:<7}{}", paint(Paint::InkSoft, label), value);
}

/// One chart row: status glyph, subject, then free-form detail.
///
/// `subject` is padded to a fixed column so detail text aligns down the page
/// the way a printed form does.
pub fn row(mark: Mark, subject: &str, detail: &str) {
    let pad = 20usize.saturating_sub(subject.chars().count());
    let subject = if mark == Mark::Absent {
        paint(Paint::InkSoft, subject)
    } else {
        subject.to_string()
    };
    println!(
        "  {}  {}{}  {}",
        mark.render(),
        subject,
        " ".repeat(pad),
        detail
    );
}

/// An indented continuation under a row — one imported artifact, one finding.
pub fn detail(text: &str) {
    println!("       {}", paint(Paint::InkSoft, text));
}

/// Closes a chart: a rule, a one-line finding, and the next command to run.
///
/// The `next` hint is the identity's first commitment — no output is a dead
/// end. Pass `None` only when the procedure genuinely terminates a workflow.
pub fn close(finding: &str, next: Option<&str>) {
    rule();
    println!("  {finding}");
    if let Some(next) = next {
        println!(
            "  {} {}",
            paint(Paint::InkSoft, "Next "),
            paint(Paint::Drape, next),
        );
    }
    println!();
}

/// Shortens a path for the subject column by stripping a known root.
///
/// Doctor findings carry absolute paths, which are far too wide for a chart
/// column. Rewriting them against the Brain and tool roots keeps the record
/// aligned and puts the reader's attention on the part that varies.
pub fn abbreviate(path: &str, roots: &[(&str, &std::path::Path)]) -> String {
    for (label, root) in roots {
        let root = root.to_string_lossy();
        if let Some(rest) = path.strip_prefix(root.as_ref()) {
            let rest = rest.trim_start_matches(['/', '\\']);
            return if rest.is_empty() {
                (*label).to_string()
            } else {
                format!("{label}/{rest}")
            };
        }
    }
    path.to_string()
}

/// Reports a failure on stderr in the identity's voice: what was being done,
/// what went wrong, and — where one exists — what to try next.
///
/// Deliberately unruled and uncoloured on stderr, so it stays readable when
/// interleaved with a chart on stdout.
pub fn fault(procedure: &str, message: &str, next: Option<&str>) {
    eprintln!();
    eprintln!("  {} halted: {message}", procedure.to_uppercase());
    if let Some(next) = next {
        eprintln!("  Try  {next}");
    }
    eprintln!();
}

/// Pluralises `noun` against `n` for the finding line — "1 tool" / "3 tools".
pub fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mark_is_exactly_one_cell_wide() {
        // Column alignment across the whole chart depends on this.
        for mark in [
            Mark::Present,
            Mark::Partial,
            Mark::Warning,
            Mark::Critical,
            Mark::Absent,
            Mark::NotApplicable,
        ] {
            assert_eq!(mark.glyph().to_string().chars().count(), 1);
        }
    }

    #[test]
    fn marks_are_visually_distinct() {
        let glyphs: Vec<char> = [
            Mark::Present,
            Mark::Partial,
            Mark::Warning,
            Mark::Critical,
            Mark::Absent,
            Mark::NotApplicable,
        ]
        .iter()
        .map(|m| m.glyph())
        .collect();
        let mut unique = glyphs.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(glyphs.len(), unique.len(), "two marks share a glyph");
    }

    #[test]
    fn no_mark_is_an_emoji() {
        // IDENTITY.md: no emoji anywhere. Emoji live above U+1F000 (and the
        // misc-symbols block); the clinical glyphs are all geometric shapes.
        for mark in [Mark::Present, Mark::Critical, Mark::Absent] {
            assert!(
                (mark.glyph() as u32) < 0x1F000,
                "{} is outside the geometric-shapes range",
                mark.glyph()
            );
        }
    }

    #[test]
    fn paint_is_transparent_when_color_is_disabled() {
        // Tests capture stdout, so colour is off and painting is a no-op.
        // This is what keeps piped output parseable.
        assert_eq!(paint(Paint::Drape, "plain"), "plain");
    }

    #[test]
    fn abbreviate_rewrites_a_known_root_to_its_label() {
        let brain = std::path::Path::new("/home/u/AIBrain");
        let roots: &[(&str, &std::path::Path)] = &[("brain", brain)];
        assert_eq!(
            abbreviate("/home/u/AIBrain/.brain/mappings.json", roots),
            "brain/.brain/mappings.json",
        );
    }

    #[test]
    fn abbreviate_collapses_the_root_itself_to_the_bare_label() {
        let brain = std::path::Path::new("/home/u/AIBrain");
        let roots: &[(&str, &std::path::Path)] = &[("brain", brain)];
        assert_eq!(abbreviate("/home/u/AIBrain", roots), "brain");
    }

    #[test]
    fn abbreviate_leaves_unrelated_paths_alone() {
        let brain = std::path::Path::new("/home/u/AIBrain");
        let roots: &[(&str, &std::path::Path)] = &[("brain", brain)];
        assert_eq!(abbreviate("/etc/hosts", roots), "/etc/hosts");
    }

    #[test]
    fn abbreviate_prefers_the_first_matching_root() {
        // Brain and tools can nest; the order of `roots` decides, so the
        // caller's precedence must be honoured rather than the longest match.
        let outer = std::path::Path::new("/home/u");
        let inner = std::path::Path::new("/home/u/AIBrain");
        let roots: &[(&str, &std::path::Path)] = &[("brain", inner), ("tools", outer)];
        assert_eq!(abbreviate("/home/u/AIBrain/skills", roots), "brain/skills");
    }

    #[test]
    fn plural_agrees_with_its_count() {
        assert_eq!(plural(0, "tool"), "0 tools");
        assert_eq!(plural(1, "tool"), "1 tool");
        assert_eq!(plural(4, "tool"), "4 tools");
    }
}
