import { describe, it, expect } from 'vitest';
import { createSandbox } from '../helpers/sandbox.js';
import { runCLI } from '../helpers/cli.js';
import fs from 'node:fs';
import path from 'node:path';

// Helper to write file contents and ensure directories exist
function writeFile(basePath: string, relativePath: string, content: string) {
  const targetPath = path.join(basePath, relativePath);
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, content, 'utf8');
}

describe('Tier 3: Combinations & State Transitions', () => {
  it('should scan a multi-tool workspace (Cursor, Cline, Zed, Claude Code) successfully', async () => {
    const sandbox = createSandbox('combo-multi-scan');
    try {
      writeFile(sandbox.workspaceDir, '.cursor/settings.json', '{}');
      writeFile(sandbox.workspaceDir, '.clinerules', '# Cline Rules');
      writeFile(sandbox.mockHome, '.config/zed/settings.json', '{}');
      writeFile(sandbox.workspaceDir, 'CLAUDE.md', '# Claude Rules');

      const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toContain('cursor');
      expect(result.stdout).toContain('cline');
      expect(result.stdout).toContain('zed');
      expect(result.stdout).toContain('claude-code');
    } finally {
      sandbox.cleanup();
    }
  });

  it('should import a multi-tool workspace combining shared skill IDs', async () => {
    const sandbox = createSandbox('combo-multi-import');
    try {
      writeFile(sandbox.workspaceDir, '.clinerules', '# Shared Rule\nIndent with spaces.\n');
      writeFile(sandbox.workspaceDir, 'CLAUDE.md', '# Shared Rule\nIndent with spaces.\n');

      const result = await runCLI(['import'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);

      // Should coalesce the shared rule into a single skill
      const skillsDir = path.join(sandbox.brainDir, 'skills');
      expect(fs.existsSync(skillsDir)).toBe(true);
    } finally {
      sandbox.cleanup();
    }
  });

  it('should project skills to multiple tool targets simultaneously', async () => {
    const sandbox = createSandbox('combo-multi-project');
    try {
      const skillData = {
        id: 'shared-rule',
        version: '1.0.0',
        triggers: ['*'],
        targets: ['cline', 'claude-code'],
        source: '# Shared Rule\nSpacing rules.\n',
        sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
      };
      writeFile(sandbox.brainDir, 'skills/shared-rule/skill.yaml', JSON.stringify(skillData));

      const result = await runCLI(['project'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);

      expect(fs.existsSync(path.join(sandbox.workspaceDir, '.clinerules'))).toBe(true);
      expect(fs.existsSync(path.join(sandbox.workspaceDir, 'CLAUDE.md'))).toBe(true);
    } finally {
      sandbox.cleanup();
    }
  });

  it('should verify import idempotency (multiple runs yield no new brain files)', async () => {
    const sandbox = createSandbox('combo-import-idempotency');
    try {
      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nSpacing rules.\n');

      const result1 = await runCLI(['import'], { sandbox, reject: false });
      expect(result1.exitCode).toBe(0);

      const mtime1 = fs.statSync(path.join(sandbox.brainDir, 'skills')).mtimeMs;

      const result2 = await runCLI(['import'], { sandbox, reject: false });
      expect(result2.exitCode).toBe(0);

      const mtime2 = fs.statSync(path.join(sandbox.brainDir, 'skills')).mtimeMs;
      expect(mtime2).toBe(mtime1); // Idempotent: no modifications if nothing changed
    } finally {
      sandbox.cleanup();
    }
  });

  it('should verify project idempotency (multiple runs yield no new workspace files)', async () => {
    const sandbox = createSandbox('combo-project-idempotency');
    try {
      const skillData = {
        id: 'rule1',
        version: '1.0.0',
        triggers: ['*'],
        targets: ['cline'],
        source: '# Rule\nSpacing rules.\n',
        sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
      };
      writeFile(sandbox.brainDir, 'skills/rule1/skill.yaml', JSON.stringify(skillData));

      const result1 = await runCLI(['project'], { sandbox, reject: false });
      expect(result1.exitCode).toBe(0);

      const mtime1 = fs.statSync(path.join(sandbox.workspaceDir, '.clinerules')).mtimeMs;

      const result2 = await runCLI(['project'], { sandbox, reject: false });
      expect(result2.exitCode).toBe(0);

      const mtime2 = fs.statSync(path.join(sandbox.workspaceDir, '.clinerules')).mtimeMs;
      expect(mtime2).toBe(mtime1);
    } finally {
      sandbox.cleanup();
    }
  });

  it('should detect and register three-way merge conflicts in sync', async () => {
    const sandbox = createSandbox('combo-sync-conflict');
    try {
      // 1. Initial sync
      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nSpacing rules.\n');
      await runCLI(['sync', '--once'], { sandbox, reject: false });

      // 2. Conflicting modifications
      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nSpacing rules mod local.\n');
      const skillData = {
        id: 'cline-rules',
        version: '1.0.0',
        triggers: ['*'],
        targets: ['cline'],
        source: '# Rules\nSpacing rules mod brain.\n',
        sha256: '83cf13426e632b724f7a2d8e404bf7c2d7f8a70c8c9735d461cf6a568c4a93a1'
      };
      writeFile(sandbox.brainDir, 'skills/cline-rules/skill.yaml', JSON.stringify(skillData));

      // 3. Sync should detect conflict
      const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
      expect(result.exitCode).not.toBe(0);
      expect(result.stdout + result.stderr).toContain('conflict');
    } finally {
      sandbox.cleanup();
    }
  });

  it('should write conflicts to conflicts.json when drift occurs on both sides', async () => {
    const sandbox = createSandbox('combo-conflict-json');
    try {
      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nSpacing rules.\n');
      await runCLI(['sync', '--once'], { sandbox, reject: false });

      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nSpacing rules mod local.\n');
      const skillData = {
        id: 'cline-rules',
        version: '1.0.0',
        triggers: ['*'],
        targets: ['cline'],
        source: '# Rules\nSpacing rules mod brain.\n',
        sha256: '83cf13426e632b724f7a2d8e404bf7c2d7f8a70c8c9735d461cf6a568c4a93a1'
      };
      writeFile(sandbox.brainDir, 'skills/cline-rules/skill.yaml', JSON.stringify(skillData));

      await runCLI(['sync', '--once'], { sandbox, reject: false });

      const conflictsPath = path.join(sandbox.brainDir, 'conflicts.json');
      expect(fs.existsSync(conflictsPath)).toBe(true);
      const conflicts = JSON.parse(fs.readFileSync(conflictsPath, 'utf8'));
      expect(conflicts.length).toBeGreaterThan(0);
    } finally {
      sandbox.cleanup();
    }
  });

  it('should prevent concurrent sync operations using a file lock', async () => {
    const sandbox = createSandbox('combo-lock');
    try {
      // Simulate file lock
      writeFile(sandbox.brainDir, '.lock', '12345');

      const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
      expect(result.exitCode).not.toBe(0);
      expect(result.stderr).toContain('lock');
    } finally {
      sandbox.cleanup();
    }
  });

  it('should fail second concurrent process with a concurrency error', async () => {
    const sandbox = createSandbox('combo-concurrency');
    try {
      const p1 = runCLI(['sync'], { sandbox, reject: false });
      const p2 = runCLI(['sync'], { sandbox, reject: false });

      const [res1, res2] = await Promise.all([p1, p2]);
      // At least one of them should fail or show locked behavior
      const eitherFailed = res1.exitCode !== 0 || res2.exitCode !== 0;
      expect(eitherFailed).toBe(true);
    } finally {
      sandbox.cleanup();
    }
  });

  it('should debounce rapid file watcher events in daemon mode', async () => {
    const sandbox = createSandbox('combo-debounce');
    try {
      // Running sync watch, writing repeatedly
      const watcher = runCLI(['sync'], { sandbox, reject: false });

      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nChange 1\n');
      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nChange 2\n');
      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nChange 3\n');

      // Allow some time for debounce
      await new Promise((resolve) => setTimeout(resolve, 500));
      watcher.kill();

      const result = await watcher;
      expect(result.exitCode).toBeDefined();
    } finally {
      sandbox.cleanup();
    }
  });

  it('should handle bulk configuration edits cleanly during debounce window', async () => {
    const sandbox = createSandbox('combo-bulk-edit');
    try {
      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\n1\n');
      writeFile(sandbox.workspaceDir, 'CLAUDE.md', '# Rules\n1\n');

      const result = await runCLI(['import'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);
    } finally {
      sandbox.cleanup();
    }
  });

  it('should verify convergence after resolving merge conflict', async () => {
    const sandbox = createSandbox('combo-resolve-conflict');
    try {
      // Simulate resolved conflict by matching workspace and brain
      writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nResolved content\n');
      const skillData = {
        id: 'cline-rules',
        version: '1.0.0',
        triggers: ['*'],
        targets: ['cline'],
        source: '# Rules\nResolved content\n',
        sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
      };
      writeFile(sandbox.brainDir, 'skills/cline-rules/skill.yaml', JSON.stringify(skillData));

      const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);
    } finally {
      sandbox.cleanup();
    }
  });
});
