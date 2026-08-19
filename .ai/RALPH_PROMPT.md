# RALPH_PROMPT.md — Inner Operation Loop

RALPH is the autonomous inner loop for the LLM Neurosurgeon project. It operates in strict read-adapt-implement-deploy cycles, ensuring systematic progress through the project plan.

## Core Algorithm (RALPH)

On every turn, RALPH:

1. **Read Phase**
   - Read PLAN.md for the remaining tasks
   - Read last 20 lines of PROGRESS.md for context
   - Read DESIGN_PACK.md only if the task is UI-related
   - Read any task-specific files (adapters, schemas, docs) per task instructions
   - Select the **SINGLE highest-priority** unchecked, unblocked task

2. **Adapt Phase**
   - Review existing code and state
   - Understand requirements from PLAN.md task description
   - Plan implementation approach, anticipating edge cases
   - Validate assumptions before implementation

3. **Implement Phase**
   - Implement the selected task
   - Fix any bugs found
   - Iterate up to 3 attempts per task
   - If still failing after 3 attempts → mark [BLOCKED] and stop

4. **Deploy Phase**
   - Run verification checks/tests defined in task
   - Git commit with conventional message
   - Tick off task in PLAN.md
   - Append status to PROGRESS.md
   - STOP (wait for next turn)

## RALPH Priority Rules
- Clear human-in-the-loop gates first (Gate 1, Gate 2, etc.)
- High-impact structural tasks before implementation details
- Cross-cutting concerns before screen-by-screen work
- Architecture and frameworks before UI polish

## Command Sequence
- Use dedicated tools (read_file, edit, write_file, glob, grep_search, run_shell_command) for all file operations
- For verification: use project-specific test commands from README, linters, and type-checkers
- All file paths must be absolute; follow project conventions strictly
- Never make assumptions; validate before editing
- Write fast, then refactor if needed

## Verification Process
- Test suites using project commands (e.g., cargo test, pnpm test)
- Linters, formatters, and type-checkers based on project stack
- Manual verification per PLAN.md task requirements
- End-to-end functional testing for cross-platform UI

## Memory & Context
- File-state over chat-state: every decision and plan lives in the repo
- Read PROGRESS.md to understand recent progress and avoid work duplication
- Remember non-blocking questions must go to QUESTIONS.md (Gate-stop rules)

## Exit Criteria
- Task is implemented and tested
- All verification requirements met
- Conventional commit created
- Progress properly recorded
- Signal that next turn's RALPH will select the new highest-priority task

## Example Implementation Pattern
```
---
On turn: read PLAN.md, last 20 lines PROGRESS.md
Task found: "T1.2 Write RALPH_PROMPT.md and full task breakdown with verify: checks."
Implementation: create RALPH_PROMPT.md with complete algorithm.
Commit: "feat(T1.2): implement RALPH operation loop with priority selection"
Progress: Append "T1.2 ✅ Implement RALPH operation loop..." to PROGRESS.md
---
```

## Note
If a task is BLOCKED after 3 attempts, the task is paused and a follow-up turn can unblock it.

RALPH ensures the project stays on a single, verified, sequentially delivered task.