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

describe('Tier 2: Boundary and Corner Cases', () => {
  // ==========================================
  // AIDER
  // ==========================================
  describe('aider boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('aider-malformed');
      try {
        writeFile(sandbox.workspaceDir, '.aider.conf.yml', 'read: [\n  broken: yaml\n');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
        expect(result.stderr).toContain('malformed');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('aider-traversal');
      try {
        writeFile(sandbox.workspaceDir, '.aider.conf.yml', 'read: ["../../../../etc/passwd"]\n');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('aider-missing');
      try {
        writeFile(sandbox.workspaceDir, '.aider.conf.yml', 'options:\n');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0); // If valid empty options
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('aider-write-protected');
      try {
        const file = path.join(sandbox.workspaceDir, 'CONVENTIONS.md');
        writeFile(sandbox.workspaceDir, 'CONVENTIONS.md', '');
        fs.chmodSync(file, 0o400); // Read-only

        const skillData = {
          id: 'conventions',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['aider'],
          source: '# Aider Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/conventions/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        // Restore permissions for cleanup
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, 'CONVENTIONS.md'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('aider-loops');
      try {
        const rulesDir = path.join(sandbox.workspaceDir, '.aider-rules');
        fs.mkdirSync(rulesDir, { recursive: true });
        fs.symlinkSync(rulesDir, path.join(rulesDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // CLAUDE-CODE
  // ==========================================
  describe('claude-code boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('claude-malformed');
      try {
        writeFile(sandbox.workspaceDir, '.claude/settings.json', '{invalid_json}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('claude-traversal');
      try {
        writeFile(sandbox.workspaceDir, '.claude/settings.json', '{"mcpServers": {"test": {"command": "../../../../bin/sh"}}}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('claude-missing');
      try {
        writeFile(sandbox.workspaceDir, '.claude/settings.json', '{}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('claude-write-protected');
      try {
        writeFile(sandbox.workspaceDir, 'CLAUDE.md', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, 'CLAUDE.md'), 0o400);

        const skillData = {
          id: 'claude-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['claude-code'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/claude-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, 'CLAUDE.md'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('claude-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.claude');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // CLINE
  // ==========================================
  describe('cline boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('cline-malformed');
      try {
        writeFile(sandbox.workspaceDir, 'cline_mcp_settings.json', '{invalid}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('cline-traversal');
      try {
        writeFile(sandbox.workspaceDir, 'cline_mcp_settings.json', '{"mcpServers": {"test": {"command": "../../../../sh"}}}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('cline-missing');
      try {
        writeFile(sandbox.workspaceDir, 'cline_mcp_settings.json', '{"mcpServers": {"sqlite": {}}}'); // command is missing
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('cline-write-protected');
      try {
        writeFile(sandbox.workspaceDir, '.clinerules', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, '.clinerules'), 0o400);

        const skillData = {
          id: 'cline-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['cline'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/cline-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, '.clinerules'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('cline-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.cline-rules');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // CONTINUE
  // ==========================================
  describe('continue boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('continue-malformed');
      try {
        writeFile(sandbox.workspaceDir, '.continue/config.json', '{invalid}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('continue-traversal');
      try {
        writeFile(sandbox.workspaceDir, '.continue/config.json', '{"mcpServers": {"test": {"command": "../../../../sh"}}}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('continue-missing');
      try {
        writeFile(sandbox.workspaceDir, '.continue/config.json', '{}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('continue-write-protected');
      try {
        writeFile(sandbox.workspaceDir, '.continue/rules/rule1.md', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, '.continue/rules/rule1.md'), 0o400);

        const skillData = {
          id: 'rule1',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['continue'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/rule1/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, '.continue/rules/rule1.md'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('continue-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.continue/rules');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // CURSOR
  // ==========================================
  describe('cursor boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('cursor-malformed');
      try {
        writeFile(sandbox.workspaceDir, '.cursor/settings.json', '{invalid}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('cursor-traversal');
      try {
        writeFile(sandbox.workspaceDir, '.cursor/rules/rule1.mdc', '---\nglobs: ../../../etc/passwd\n---\n# Traversal\n');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('cursor-missing');
      try {
        writeFile(sandbox.workspaceDir, '.cursor/rules/rule1.mdc', '---\n# Missing frontmatter keys\n---\nRule content');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('cursor-write-protected');
      try {
        writeFile(sandbox.workspaceDir, '.cursor/rules/rule1.mdc', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, '.cursor/rules/rule1.mdc'), 0o400);

        const skillData = {
          id: 'rule1',
          version: '1.0.0',
          triggers: ['*.ts'],
          targets: ['cursor'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/rule1/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, '.cursor/rules/rule1.mdc'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('cursor-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.cursor/rules');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // GEMINI-CLI
  // ==========================================
  describe('gemini-cli boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('gemini-malformed');
      try {
        writeFile(sandbox.workspaceDir, '.gemini/settings.json', '{invalid}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('gemini-traversal');
      try {
        writeFile(sandbox.workspaceDir, '.gemini/settings.json', '{"mcpServers": {"test": {"command": "../../../../sh"}}}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('gemini-missing');
      try {
        writeFile(sandbox.workspaceDir, '.gemini/settings.json', '{}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('gemini-write-protected');
      try {
        writeFile(sandbox.workspaceDir, 'GEMINI.md', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, 'GEMINI.md'), 0o400);

        const skillData = {
          id: 'gemini-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['gemini-cli'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/gemini-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, 'GEMINI.md'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('gemini-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.gemini');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // GITHUB-COPILOT
  // ==========================================
  describe('github-copilot boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('copilot-malformed');
      try {
        writeFile(sandbox.workspaceDir, '.vscode/mcp.json', '{invalid}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('copilot-traversal');
      try {
        writeFile(sandbox.workspaceDir, '.vscode/mcp.json', '{"mcpServers": {"test": {"command": "../../../../sh"}}}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('copilot-missing');
      try {
        writeFile(sandbox.workspaceDir, '.vscode/mcp.json', '{"mcpServers": {"test": {}}}'); // command missing
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('copilot-write-protected');
      try {
        writeFile(sandbox.workspaceDir, '.github/copilot-instructions.md', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, '.github/copilot-instructions.md'), 0o400);

        const skillData = {
          id: 'github-copilot-instructions',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['github-copilot'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/github-copilot-instructions/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, '.github/copilot-instructions.md'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('copilot-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.github');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // OPENAI-CODEX
  // ==========================================
  describe('openai-codex boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('codex-malformed');
      try {
        writeFile(sandbox.mockHome, '.codex/config.json', '{invalid}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('codex-traversal');
      try {
        writeFile(sandbox.mockHome, '.codex/config.json', '{"command": "../../../../sh"}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('codex-missing');
      try {
        writeFile(sandbox.mockHome, '.codex/config.json', '{}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('codex-write-protected');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, 'AGENTS.md'), 0o400);

        const skillData = {
          id: 'codex-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['openai-codex'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/codex-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, 'AGENTS.md'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('codex-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.codex');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // OPENCODE
  // ==========================================
  describe('opencode boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('opencode-malformed');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '<!-- malformed comment -->\n```yaml\n- broken\n');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('opencode-traversal');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '<!-- path traversal -->\n[External](../../../../etc/passwd)');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('opencode-missing');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# OpenCode Agents\n');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('opencode-write-protected');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, 'AGENTS.md'), 0o400);

        const skillData = {
          id: 'opencode-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['opencode'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/opencode-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, 'AGENTS.md'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('opencode-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.opencode');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // ROO-CODE
  // ==========================================
  describe('roo-code boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('roo-malformed');
      try {
        writeFile(sandbox.workspaceDir, '.roomodes', '{invalid}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('roo-traversal');
      try {
        writeFile(sandbox.workspaceDir, '.roomodes', '{"customModes": [{"trigger": "../../../../sh"}]}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('roo-missing');
      try {
        writeFile(sandbox.workspaceDir, '.roomodes', '{}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('roo-write-protected');
      try {
        writeFile(sandbox.workspaceDir, '.clinerules', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, '.clinerules'), 0o400);

        const skillData = {
          id: 'roo-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['roo-code'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/roo-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, '.clinerules'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('roo-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.roo');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // WINDSURF
  // ==========================================
  describe('windsurf boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('windsurf-malformed');
      try {
        writeFile(sandbox.mockHome, '.codeium/windsurf/mcp.json', '{invalid}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('windsurf-traversal');
      try {
        writeFile(sandbox.mockHome, '.codeium/windsurf/mcp.json', '{"mcpServers": {"test": {"command": "../../../../sh"}}}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('windsurf-missing');
      try {
        writeFile(sandbox.mockHome, '.codeium/windsurf/mcp.json', '{"mcpServers": {"sqlite": {}}}'); // command missing
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('windsurf-write-protected');
      try {
        writeFile(sandbox.workspaceDir, '.windsurfrules', '');
        fs.chmodSync(path.join(sandbox.workspaceDir, '.windsurfrules'), 0o400);

        const skillData = {
          id: 'windsurf-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['windsurf'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/windsurf-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.workspaceDir, '.windsurfrules'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('windsurf-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.codeium');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // ZED
  // ==========================================
  describe('zed boundary cases', () => {
    it('should handle malformed config gracefully', async () => {
      const sandbox = createSandbox('zed-malformed');
      try {
        writeFile(sandbox.mockHome, '.config/zed/settings.json', '{invalid}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should prevent path traversal attacks', async () => {
      const sandbox = createSandbox('zed-traversal');
      try {
        writeFile(sandbox.mockHome, '.config/zed/settings.json', '{"context_servers": {"test": {"command": "../../../../sh"}}}');
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should handle missing fields in configuration', async () => {
      const sandbox = createSandbox('zed-missing');
      try {
        writeFile(sandbox.mockHome, '.config/zed/settings.json', '{"context_servers": {"sqlite": {}}}'); // command missing
        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should fail gracefully on write protection', async () => {
      const sandbox = createSandbox('zed-write-protected');
      try {
        writeFile(sandbox.mockHome, '.config/zed/AGENTS.md', '');
        fs.chmodSync(path.join(sandbox.mockHome, '.config/zed/AGENTS.md'), 0o400);

        const skillData = {
          id: 'zed-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['zed'],
          source: '# Rules\nUpdate.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/zed-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).not.toBe(0);
      } finally {
        try {
          fs.chmodSync(path.join(sandbox.mockHome, '.config/zed/AGENTS.md'), 0o600);
        } catch {}
        sandbox.cleanup();
      }
    });

    it('should detect and prevent loops in directory structures', async () => {
      const sandbox = createSandbox('zed-loops');
      try {
        const targetDir = path.join(sandbox.workspaceDir, '.config');
        fs.mkdirSync(targetDir, { recursive: true });
        fs.symlinkSync(targetDir, path.join(targetDir, 'loop'));

        const result = await runCLI(['scan'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });
});
