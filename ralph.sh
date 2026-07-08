#!/usr/bin/env bash
#
# ralph.sh — the RALPH outer loop.
#
# Repeatedly invokes `claude -p "$(cat RALPH_PROMPT.md)"`. Each iteration the
# agent reads PLAN.md, picks the single highest-priority unchecked, non-gate
# task, implements it, runs its `verify:` check, commits, and ticks the box.
# Fresh context per iteration is the token saver (see MASTER_PROMPT.md §2).
#
# The loop stops when any of these is true:
#   - no unchecked non-gate tasks remain in PLAN.md               (done)
#   - the next unchecked task is a human GATE                     (needs review)
#   - an iteration completed no task (a task went [BLOCKED])      (avoid a spin)
#   - MAX_ITERATIONS is reached                                   (safety cap)
#   - the claude CLI exits non-zero                               (error)
#
# Usage:
#   ./ralph.sh                      # run until a gate / done (default 20 iters)
#   MAX_ITERATIONS=5 ./ralph.sh     # cap the number of iterations
#   PERMISSION_MODE=plan ./ralph.sh # dry-plan instead of editing (acceptEdits default)
#   DRY_RUN=1 ./ralph.sh            # print what would run; don't call claude
#   CLAUDE_BIN=/path/to/claude ./ralph.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT" || exit 1

PLAN="$ROOT/PLAN.md"
PROMPT_FILE="$ROOT/RALPH_PROMPT.md"
MAX_ITERATIONS="${MAX_ITERATIONS:-20}"
PERMISSION_MODE="${PERMISSION_MODE:-acceptEdits}"
CLAUDE_BIN="${CLAUDE_BIN:-claude}"
DRY_RUN="${DRY_RUN:-0}"

for f in "$PLAN" "$PROMPT_FILE"; do
  if [[ ! -f "$f" ]]; then
    echo "ralph: required file not found: $f" >&2
    exit 1
  fi
done

if [[ "$DRY_RUN" != "1" ]] && ! command -v "$CLAUDE_BIN" >/dev/null 2>&1; then
  echo "ralph: '$CLAUDE_BIN' not found on PATH. Install Claude Code or set CLAUDE_BIN." >&2
  exit 127
fi

# Number of unchecked tasks that are NOT human gates.
open_tasks() {
  local n
  n="$(grep -E '^- \[ \]' "$PLAN" 2>/dev/null | grep -vci 'gate' || true)"
  echo "${n:-0}"
}

# The first unchecked task line in file order (used for gate detection).
next_task() {
  grep -E '^- \[ \]' "$PLAN" 2>/dev/null | head -n 1 || true
}

iter=0
while :; do
  remaining="$(open_tasks)"
  nxt="$(next_task)"

  if [[ "$remaining" -eq 0 ]]; then
    echo "ralph: no unchecked non-gate tasks left in PLAN.md — done."
    break
  fi

  if [[ -n "$nxt" ]] && grep -qiE '^- \[ \].*gate' <<<"$nxt"; then
    echo "ralph: next task is a human GATE — stopping for review:"
    echo "       $nxt"
    break
  fi

  if [[ "$iter" -ge "$MAX_ITERATIONS" ]]; then
    echo "ralph: reached MAX_ITERATIONS=$MAX_ITERATIONS — stopping."
    break
  fi

  iter=$((iter + 1))
  echo "──── RALPH iteration $iter (open non-gate tasks: $remaining) ────"
  echo "     next: $nxt"

  if [[ "$DRY_RUN" == "1" ]]; then
    echo "     [dry-run] would run:" \
      "$CLAUDE_BIN -p \"\$(cat RALPH_PROMPT.md)\" --permission-mode $PERMISSION_MODE"
    break
  fi

  before="$remaining"
  if ! "$CLAUDE_BIN" -p "$(cat "$PROMPT_FILE")" --permission-mode "$PERMISSION_MODE"; then
    echo "ralph: claude exited non-zero on iteration $iter — stopping." >&2
    exit 1
  fi

  after="$(open_tasks)"
  if [[ "$after" -ge "$before" ]]; then
    echo "ralph: no task was completed this iteration (a task may be [BLOCKED]) — stopping to avoid a spin." >&2
    break
  fi
done

echo "ralph: loop finished after $iter iteration(s)."
