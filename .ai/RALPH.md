# RALPH LOOP

Read PLAN.md + last 20 lines of PROGRESS.md + DESIGN_PACK.md (if UI task). Select SINGLE highest-priority unchecked task. Implement it. Verify: run test/check. 3 attempts → [BLOCKED] & stop. git commit with conventional message, tick box, append to PROGRESS.md. STOP.

## RALPH Algorithm (Read-Adapt-Implement-Deploy)

### 1. Read Phase
- Read PLAN.md for the remaining tasks
- Read last 20 lines of PROGRESS.md for context
- Read DESIGN_PACK.md for UI-related tasks
- Select the SINGLE highest-priority, unchecked, unblocked task

### 2. Adapt Phase
- Review existing code & state
- Understand the requirements
- Plan implementation approach
- Anticipate edge cases and blockers

### 3. Implement Phase
- Implement the selected task
- Fix any bugs or issues found
- Iterate up to 3 attempts
- If still failing after 3 attempts → mark [BLOCKED] and stop

### 4. Deploy Phase
- Run verification checks/tests
- Git commit with conventional message
- Tick off task in PLAN.md
- Append status to PROGRESS.md

## Task Prioritization Rules
1. Clear human-in-the-loop gates first (Gate 1, Gate 2, etc.)
2. High-impact structural tasks before implementation details
3. Cross-cutting concerns before screen-by-screen work
4. Architecture and frameworks before UI polish

## Implementation Rules
- Follow project conventions: prefer file editing over creating new files
- Use absolute paths when editing files
- Maintain existing style, naming, and typing patterns
- Keep changes focused and idempotent
- Avoid premature abstractions and single responsibility violations
- Validate assumptions before implementation
- Write fast, then refactor

## Verification
- Run test suites using project-specific commands from README
- Use linters, formatters, and type-checkers as appropriate for this project
- Check for completion according to PLAN.md verification criteria
- Ensure everything works end-to-end before committing

## Exit Criteria
- Task is implemented and tested
- All verification passes
- Conventional commit made
- Progress recorded
- Ready for next task selection