# Proposed LLM Neurosurgeon E2E Testing Infrastructure

This document outlines the testing infrastructure, directory layout, environment configuration, and test suites for the LLM Neurosurgeon End-to-End (E2E) testing suite.

---

## 1. Directory Structure

The E2E tests are written in TypeScript using **Vitest** and are integrated into the monorepo as a standalone package under `packages/e2e/` within the PNPM workspace.

```
packages/e2e/
├── package.json               # Package configuration & dependencies
├── tsconfig.json              # TypeScript compilation rules
├── vitest.config.ts           # Vitest configuration (setup/teardown files)
├── globalSetup.ts             # Global setup hook (builds the Rust CLI binary)
├── src/
│   ├── helpers/
│   │   ├── sandbox.ts         # Sandbox manager (temp dirs, env variables, fixtures)
│   │   ├── cli.ts             # Spawn and execute the Rust CLI binary
│   │   └── schema.ts          # Validates files against packages/schema
│   └── suites/
│       ├── tier1_feature.test.ts      # Tier 1: Happy-path feature coverage
│       ├── tier2_boundary.test.ts     # Tier 2: Boundary and corner cases
│       ├── tier3_combination.test.ts  # Tier 3: Multi-tool sync and transitions
│       └── tier4_workload.test.ts     # Tier 4: Real-world workloads and migrations
```

---

## 2. Test Runner Execution & Configuration

### Commands

To run all E2E tests, build the CLI first and execute Vitest:
```bash
# Run all tests
pnpm --filter @llm-neurosurgeon/e2e test

# Run tests in watch mode
pnpm --filter @llm-neurosurgeon/e2e test --watch

# Run a specific tier's tests
pnpm --filter @llm-neurosurgeon/e2e test src/suites/tier1_feature.test.ts
```

### Environment Isolation

To ensure that E2E tests do not mutate the developer's actual home directory or project directory, the test runner isolates execution using the following environment variables passed to the child process:

- `HOME`: Set to a temporary mock directory (isolates global configs like `~/.claude/settings.json`).
- `NEUROSURGEON_BRAIN_PATH`: Pointed to the sandboxed `AIBrain/` directory.
- `NEUROSURGEON_WORKSPACE_PATH`: Pointed to the sandboxed `workspace/` directory containing simulated tool configs.
- `NEUROSURGEON_LOG_DIR`: Pointed to the temporary directory's log folder.

---

## 3. Sandbox Infrastructure

Every test runs inside a sandboxed environment managed by `src/helpers/sandbox.ts`.

```typescript
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

export interface Sandbox {
  tmpDir: string;
  workspaceDir: string;
  brainDir: string;
  mockHome: string;
  cleanup: () => void;
}

export function createSandbox(testName: string): Sandbox {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), `neuro-e2e-${testName}-`));
  const workspaceDir = path.join(tmpDir, 'workspace');
  const brainDir = path.join(tmpDir, 'AIBrain');
  const mockHome = path.join(tmpDir, 'home');

  fs.mkdirSync(workspaceDir);
  fs.mkdirSync(brainDir);
  fs.mkdirSync(mockHome);

  const cleanup = () => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  };

  return { tmpDir, workspaceDir, brainDir, mockHome, cleanup };
}
```

---

## 4. 4-Tier Test Suite Specification

### Tier 1: Feature Coverage (Sanity & Happy Path)

Verifies standard operation of the core CLI verbs in isolation.

1. **`scan` Happy Path**
   - **Setup:** Populate `workspace/` with a valid `.cursorrules` and `.cursor/rules/repo-conventions.mdc` fixture.
   - **Action:** Execute `neurosurgeon scan --json`.
   - **Assert:** Exit code is 0. Standard output is a JSON array containing a record for `"cursor"` with correct path properties.
2. **`import` Happy Path**
   - **Setup:** Same as scan.
   - **Action:** Execute `neurosurgeon import` (without `--dry-run`).
   - **Assert:** Canonical `AIBrain/skills/repo-conventions/skill.yaml` and `AIBrain/skills/repo-conventions/SKILL.md` are created. Files match structural schemas.
3. **`project` Happy Path**
   - **Setup:** Populate `AIBrain/` with canonical skill and agent files.
   - **Action:** Execute `neurosurgeon project`.
   - **Assert:** The matching files are written or symlinked into the corresponding paths in `workspace/` (e.g. `.cursor/rules/repo-conventions.mdc` recreated).
4. **`sync` Happy Path**
   - **Setup:** A valid hybrid workspace.
   - **Action:** Execute `neurosurgeon sync --once`.
   - **Assert:** Both scanning, importing, and projecting occur, resolving dependencies and ensuring state is unified.
5. **`doctor` Happy Path**
   - **Setup:** A target file in `workspace/` is modified directly (creating drift between workspace and Brain).
   - **Action:** Run `neurosurgeon doctor`. Verify it reports the drift. Run `neurosurgeon doctor --fix` and verify it restores alignment.
6. **`snapshot` & `rollback` Happy Path**
   - **Setup:** A clean Brain.
   - **Action:** Run `neurosurgeon snapshot "initial state"`. Modify brain contents. Run `neurosurgeon rollback <ref>`.
   - **Assert:** Files are restored to the exact initial byte structure.

---

### Tier 2: Boundary & Corner Cases (Edge Cases & Fault Tolerance)

Verifies how the system responds to environment variance, errors, and potential security vulnerabilities.

1. **Path Traversal & Symlink Escapes (Security Gate)**
   - **Setup:** Inject a custom skill with `source` targeting `../../../../etc/passwd` or a symlink escape in `workspace/`.
   - **Action:** Run `neurosurgeon import`.
   - **Assert:** The CLI rejects the path traversal, exits with a non-zero code, and writes no files outside the sandboxed `AIBrain` or `workspace`.
2. **Circular Symlink Loops**
   - **Setup:** In `workspace/`, create a directory structure where `.cursor/rules/loop` is a symlink pointing to `.cursor/rules`.
   - **Action:** Run `neurosurgeon scan`.
   - **Assert:** The scanner terminates successfully (does not hang or overflow the stack) and reports valid configurations without looping.
3. **Malformed Input Files**
   - **Setup:** Create a `workspace/` config with broken YAML syntax (e.g. invalid indentations, duplicate keys) or corrupted Markdown frontmatter.
   - **Action:** Run `neurosurgeon import`.
   - **Assert:** CLI exits with status code 1. Output error message contains `malformed config: <filename>` and wraps the parser error cleanly (no panic/unhandled error).
4. **Schema Violation Prevention**
   - **Setup:** Inject a skill file into `AIBrain/` with missing required keys (e.g., missing `sha256` or `version`) or containing unknown properties.
   - **Action:** Run `neurosurgeon project`.
   - **Assert:** CLI detects the schema violation, reports it, and exits with a non-zero status code without modifying workspace files.
5. **Write-Protected Filesystem**
   - **Setup:** Mark `workspace/` as read-only (chmod 400/500).
   - **Action:** Run `neurosurgeon project`.
   - **Assert:** Exits with standard `AdapterError::Io` error message. No half-written or corrupted configuration remnants.
6. **No-Symlink Platform Simulation**
   - **Setup:** Run tests simulating a Windows environment where symlink privileges are unavailable.
   - **Action:** Run `neurosurgeon project` (under the `Decide` rule).
   - **Assert:** Projector falls back automatically to `Generate` policy, copying the file with the appropriate provenance stamp.

---

### Tier 3: Combinations & State Transitions (Sync, Merge, & Conflict Resolution)

Verifies coordination of multiple adapters and concurrent operations.

1. **Multi-Tool Workspace Integration**
   - **Setup:** A workspace containing configurations for four tools simultaneously (Cursor, Cline, Zed, and Claude Code).
   - **Action:** Run `neurosurgeon scan` and `neurosurgeon import`.
   - **Assert:** All four configurations are detected and combined into the canonical Brain model, sharing skills when IDs align.
2. **Idempotency & Convergence**
   - **Setup:** A dirty workspace.
   - **Action:** Run `import` followed by `project`, followed by `import` and `project` again.
   - **Assert:** The second run produces zero writes or symlinks (byte-for-byte identical, stable state, no infinite loops).
3. **Three-Way Merge Conflict & Queue**
   - **Setup:**
     - User edits a rule in `AIBrain/skills/lint/skill.yaml`.
     - Simultaneously, user edits the corresponding target rule file directly in `.cursor/rules/lint.mdc`.
   - **Action:** Run `neurosurgeon sync`.
   - **Assert:** Sync returns `ConflictQueued`. The conflict is registered in the review file (e.g., `AIBrain/conflicts.json`), and the actual rules files are left untouched.
4. **Concurrency Locking**
   - **Setup:** Set up a slow import process (or simulate a large database operation).
   - **Action:** Spawn two `neurosurgeon sync` processes simultaneously in the same sandbox.
   - **Assert:** The second process fails immediately with a descriptive lock-error code, ensuring that concurrent operations do not corrupt state.
5. **Debouncing Watcher Events**
   - **Setup:** Mock the filesystem watcher. Trigger 50 rapid write events on rule files within a 50ms window.
   - **Action:** Run watcher service.
   - **Assert:** The watcher debounces the requests, running exactly 1 sync invocation after a quiet period of 300ms.

---

### Tier 4: Real-world Workloads & End-to-End User Scenarios

Verifies long-term viability and performance under production loads.

1. **Legacy to Modular Migration Flow**
   - **Setup:** A workspace with a legacy `.cursorrules` file containing multiple rules combined.
   - **Action:** Run `import`, delete `.cursorrules`, and run `project`.
   - **Assert:** Modular rule files are correctly created in `.cursor/rules/*.mdc`, extracting rules into structured YAML/Markdown skills in the Brain.
2. **High Volume Scaling**
   - **Setup:** A mock `AIBrain/` populated with 150 skills, 50 agents, and 30 MCP servers.
   - **Action:** Run `neurosurgeon project` and measure execution metrics.
   - **Assert:** Total execution time is under 150ms. Memory consumption is under 50MB. All files are projected correctly.
3. **Cross-Platform Compatibility (Path/LF Normalization)**
   - **Setup:** Import paths created on Windows using backslashes and CRLF line endings.
   - **Action:** Sync and project on Linux.
   - **Assert:** File hashes are verified based on normalized LF text, and directory separation slashes are normalized to forward slashes in `skill.yaml` without generating false-drift warnings.
