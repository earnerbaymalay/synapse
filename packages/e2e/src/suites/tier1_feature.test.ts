import { describe, it, expect } from 'vitest';
import { createSandbox } from '../helpers/sandbox.js';
import { runCLI } from '../helpers/cli.js';
import { validateFile } from '../helpers/schema.js';
import fs from 'node:fs';
import path from 'node:path';

// Helper to write file contents and ensure directories exist
function writeFile(basePath: string, relativePath: string, content: string) {
  const targetPath = path.join(basePath, relativePath);
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.writeFileSync(targetPath, content, 'utf8');
}

describe('Tier 1: Happy-path Feature Coverage', () => {
  // ==========================================
  // AIDER
  // ==========================================
  describe('aider adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('aider-detect');
      try {
        writeFile(sandbox.workspaceDir, '.aider.conf.yml', 'read: [CONVENTIONS.md]\n');
        writeFile(sandbox.workspaceDir, 'CONVENTIONS.md', '# Conventions\n');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('aider');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('aider-import');
      try {
        writeFile(sandbox.workspaceDir, '.aider.conf.yml', 'read: [CONVENTIONS.md]\n');
        writeFile(sandbox.workspaceDir, 'CONVENTIONS.md', '# Conventions\nUse tabs.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/conventions/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
        expect(validateFile(skillYamlPath, 'skill').valid).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('aider-project');
      try {
        const skillData = {
          id: 'conventions',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['aider'],
          source: '# Aider Rules\nAlways double-check coding style.\n',
          sha256: '83cf13426e632b724f7a2d8e404bf7c2d7f8a70c8c9735d461cf6a568c4a93a1'
        };
        writeFile(sandbox.brainDir, 'skills/conventions/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const conventionPath = path.join(sandbox.workspaceDir, 'CONVENTIONS.md');
        expect(fs.existsSync(conventionPath)).toBe(true);
        expect(fs.readFileSync(conventionPath, 'utf8')).toContain('Always double-check');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('aider-sync');
      try {
        writeFile(sandbox.workspaceDir, '.aider.conf.yml', 'read: [CONVENTIONS.md]\n');
        writeFile(sandbox.workspaceDir, 'CONVENTIONS.md', '# Conventions\nUse tabs.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('aider-doctor');
      try {
        writeFile(sandbox.workspaceDir, '.aider.conf.yml', 'read: [CONVENTIONS.md]\n');
        writeFile(sandbox.workspaceDir, 'CONVENTIONS.md', '# Conventions\nUse tabs.\n');

        const scanRes = await runCLI(['import'], { sandbox, reject: false });
        expect(scanRes.exitCode).toBe(0);

        writeFile(sandbox.workspaceDir, 'CONVENTIONS.md', '# Conventions\nUse spaces.\n');

        const doctorRes = await runCLI(['doctor'], { sandbox, reject: false });
        expect(doctorRes.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // CLAUDE-CODE
  // ==========================================
  describe('claude-code adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('claude-detect');
      try {
        writeFile(sandbox.workspaceDir, '.claude/settings.json', '{"theme": "dark"}');
        writeFile(sandbox.workspaceDir, 'CLAUDE.md', '# Claude Conventions\n');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('claude-code');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('claude-import');
      try {
        writeFile(sandbox.workspaceDir, 'CLAUDE.md', '# Rules\nUse semicolons.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/claude-rules/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('claude-project');
      try {
        const skillData = {
          id: 'claude-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['claude-code'],
          source: '# Rules\nAlways document public APIs.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/claude-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, 'CLAUDE.md');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Always document public APIs');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('claude-sync');
      try {
        writeFile(sandbox.workspaceDir, 'CLAUDE.md', '# Rules\nUse semicolons.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('claude-doctor');
      try {
        writeFile(sandbox.workspaceDir, 'CLAUDE.md', '# Rules\nUse semicolons.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, 'CLAUDE.md', '# Rules\nNo semicolons.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // CLINE
  // ==========================================
  describe('cline adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('cline-detect');
      try {
        writeFile(sandbox.workspaceDir, '.clinerules', '# Cline Rules\n');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('cline');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('cline-import');
      try {
        writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nWrite tests first.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/cline-rules/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('cline-project');
      try {
        const skillData = {
          id: 'cline-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['cline'],
          source: '# Cline Rules\nTests first.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/cline-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, '.clinerules');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Tests first');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('cline-sync');
      try {
        writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nWrite tests first.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('cline-doctor');
      try {
        writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nWrite tests first.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nDon\'t write tests.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // CONTINUE
  // ==========================================
  describe('continue adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('continue-detect');
      try {
        writeFile(sandbox.workspaceDir, '.continue/config.json', '{"mcpServers": {}}');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('continue');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('continue-import');
      try {
        writeFile(sandbox.workspaceDir, '.continue/rules/rule1.md', '---\nid: rule1\n---\n# Rule\nStrict typing.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/rule1/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('continue-project');
      try {
        const skillData = {
          id: 'rule1',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['continue'],
          source: '# Rule\nStrict typing.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/rule1/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, '.continue/rules/rule1.md');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Strict typing');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('continue-sync');
      try {
        writeFile(sandbox.workspaceDir, '.continue/rules/rule1.md', '# Rule\nStrict typing.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('continue-doctor');
      try {
        writeFile(sandbox.workspaceDir, '.continue/rules/rule1.md', '# Rule\nStrict typing.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, '.continue/rules/rule1.md', '# Rule\nLoose typing.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // CURSOR
  // ==========================================
  describe('cursor adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('cursor-detect');
      try {
        writeFile(sandbox.workspaceDir, '.cursor/settings.json', '{"assistant": {}}');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('cursor');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('cursor-import');
      try {
        writeFile(sandbox.workspaceDir, '.cursor/rules/rule1.mdc', '---\ndescription: My rule\nglobs: *.ts\n---\n# Rule\nStrict.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/rule1/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('cursor-project');
      try {
        const skillData = {
          id: 'rule1',
          version: '1.0.0',
          triggers: ['*.ts'],
          targets: ['cursor'],
          source: '# Rule\nStrict.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/rule1/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, '.cursor/rules/rule1.mdc');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Strict');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('cursor-sync');
      try {
        writeFile(sandbox.workspaceDir, '.cursor/rules/rule1.mdc', '---\ndescription: My rule\nglobs: *.ts\n---\n# Rule\nStrict.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('cursor-doctor');
      try {
        writeFile(sandbox.workspaceDir, '.cursor/rules/rule1.mdc', '---\ndescription: My rule\nglobs: *.ts\n---\n# Rule\nStrict.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, '.cursor/rules/rule1.mdc', '---\ndescription: My rule\nglobs: *.ts\n---\n# Rule\nLoose.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // GEMINI-CLI
  // ==========================================
  describe('gemini-cli adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('gemini-detect');
      try {
        writeFile(sandbox.workspaceDir, '.gemini/settings.json', '{"model": "gemini-1.5-pro"}');
        writeFile(sandbox.workspaceDir, 'GEMINI.md', '# Gemini context\n');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('gemini-cli');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('gemini-import');
      try {
        writeFile(sandbox.workspaceDir, 'GEMINI.md', '# Rules\nOptimize for memory.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/gemini-rules/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('gemini-project');
      try {
        const skillData = {
          id: 'gemini-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['gemini-cli'],
          source: '# Rules\nOptimize for memory.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/gemini-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, 'GEMINI.md');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Optimize for memory');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('gemini-sync');
      try {
        writeFile(sandbox.workspaceDir, 'GEMINI.md', '# Rules\nOptimize for memory.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('gemini-doctor');
      try {
        writeFile(sandbox.workspaceDir, 'GEMINI.md', '# Rules\nOptimize for memory.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, 'GEMINI.md', '# Rules\nOptimize for speed.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // GITHUB-COPILOT
  // ==========================================
  describe('github-copilot adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('copilot-detect');
      try {
        writeFile(sandbox.workspaceDir, '.github/copilot-instructions.md', '# Copilot conventions\n');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('github-copilot');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('copilot-import');
      try {
        writeFile(sandbox.workspaceDir, '.github/copilot-instructions.md', '# Rules\nWrite comments.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/github-copilot-instructions/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('copilot-project');
      try {
        const skillData = {
          id: 'github-copilot-instructions',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['github-copilot'],
          source: '# Rules\nWrite comments.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/github-copilot-instructions/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, '.github/copilot-instructions.md');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Write comments');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('copilot-sync');
      try {
        writeFile(sandbox.workspaceDir, '.github/copilot-instructions.md', '# Rules\nWrite comments.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('copilot-doctor');
      try {
        writeFile(sandbox.workspaceDir, '.github/copilot-instructions.md', '# Rules\nWrite comments.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, '.github/copilot-instructions.md', '# Rules\nNo comments.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // OPENAI-CODEX
  // ==========================================
  describe('openai-codex adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('codex-detect');
      try {
        writeFile(sandbox.mockHome, '.codex/config.json', '{"engine": "codex"}');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('openai-codex');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('codex-import');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# Rules\nUse functional programming.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/codex-rules/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('codex-project');
      try {
        const skillData = {
          id: 'codex-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['openai-codex'],
          source: '# Rules\nUse functional programming.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/codex-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, 'AGENTS.md');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Use functional programming');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('codex-sync');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# Rules\nUse functional programming.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('codex-doctor');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# Rules\nUse functional programming.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# Rules\nUse OOP.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // OPENCODE
  // ==========================================
  describe('opencode adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('opencode-detect');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# OpenCode Agents\n');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('opencode');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('opencode-import');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# Rules\nValidate everything.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/opencode-rules/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('opencode-project');
      try {
        const skillData = {
          id: 'opencode-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['opencode'],
          source: '# Rules\nValidate everything.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/opencode-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, 'AGENTS.md');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Validate everything');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('opencode-sync');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# Rules\nValidate everything.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('opencode-doctor');
      try {
        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# Rules\nValidate everything.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, 'AGENTS.md', '# Rules\nSkip validation.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // ROO-CODE
  // ==========================================
  describe('roo-code adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('roomodes-detect');
      try {
        writeFile(sandbox.workspaceDir, '.roomodes', '{"customModes": []}');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('roo-code');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('roomodes-import');
      try {
        writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nAvoid recursion.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/roo-rules/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('roomodes-project');
      try {
        const skillData = {
          id: 'roo-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['roo-code'],
          source: '# Rules\nAvoid recursion.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/roo-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, '.clinerules');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Avoid recursion');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('roomodes-sync');
      try {
        writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nAvoid recursion.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('roomodes-doctor');
      try {
        writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nAvoid recursion.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, '.clinerules', '# Rules\nRecursion ok.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // WINDSURF
  // ==========================================
  describe('windsurf adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('windsurf-detect');
      try {
        writeFile(sandbox.workspaceDir, '.windsurfrules', '# Windsurf Rules\n');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('windsurf');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('windsurf-import');
      try {
        writeFile(sandbox.workspaceDir, '.windsurfrules', '# Rules\nPrefer lightweight models.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/windsurf-rules/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('windsurf-project');
      try {
        const skillData = {
          id: 'windsurf-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['windsurf'],
          source: '# Rules\nPrefer lightweight models.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/windsurf-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.workspaceDir, '.windsurfrules');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Prefer lightweight models');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('windsurf-sync');
      try {
        writeFile(sandbox.workspaceDir, '.windsurfrules', '# Rules\nPrefer lightweight models.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('windsurf-doctor');
      try {
        writeFile(sandbox.workspaceDir, '.windsurfrules', '# Rules\nPrefer lightweight models.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.workspaceDir, '.windsurfrules', '# Rules\nPrefer heavy models.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });

  // ==========================================
  // ZED
  // ==========================================
  describe('zed adapter', () => {
    it('should detect the tool configuration via scan', async () => {
      const sandbox = createSandbox('zed-detect');
      try {
        writeFile(sandbox.mockHome, '.config/zed/settings.json', '{"theme": "One Dark"}');

        const result = await runCLI(['scan', '--json'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
        expect(result.stdout).toContain('zed');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should import the tool rules/skills into the canonical brain', async () => {
      const sandbox = createSandbox('zed-import');
      try {
        writeFile(sandbox.mockHome, '.config/zed/AGENTS.md', '# Rules\nWrite clean comments.\n');

        const result = await runCLI(['import'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const skillYamlPath = path.join(sandbox.brainDir, 'skills/zed-rules/skill.yaml');
        expect(fs.existsSync(skillYamlPath)).toBe(true);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should project the canonical brain skills to the tool layout', async () => {
      const sandbox = createSandbox('zed-project');
      try {
        const skillData = {
          id: 'zed-rules',
          version: '1.0.0',
          triggers: ['*'],
          targets: ['zed'],
          source: '# Rules\nWrite clean comments.\n',
          sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
        };
        writeFile(sandbox.brainDir, 'skills/zed-rules/skill.yaml', JSON.stringify(skillData));

        const result = await runCLI(['project'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const rulePath = path.join(sandbox.mockHome, '.config/zed/AGENTS.md');
        expect(fs.existsSync(rulePath)).toBe(true);
        expect(fs.readFileSync(rulePath, 'utf8')).toContain('Write clean comments');
      } finally {
        sandbox.cleanup();
      }
    });

    it('should sync rules and configs for the tool', async () => {
      const sandbox = createSandbox('zed-sync');
      try {
        writeFile(sandbox.mockHome, '.config/zed/AGENTS.md', '# Rules\nWrite clean comments.\n');

        const result = await runCLI(['sync', '--once'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });

    it('should detect and fix drift for the tool via doctor', async () => {
      const sandbox = createSandbox('zed-doctor');
      try {
        writeFile(sandbox.mockHome, '.config/zed/AGENTS.md', '# Rules\nWrite clean comments.\n');
        await runCLI(['import'], { sandbox, reject: false });

        writeFile(sandbox.mockHome, '.config/zed/AGENTS.md', '# Rules\nDon\'t write comments.\n');

        const result = await runCLI(['doctor'], { sandbox, reject: false });
        expect(result.exitCode).toBe(0);

        const fixRes = await runCLI(['doctor', '--fix'], { sandbox, reject: false });
        expect(fixRes.exitCode).toBe(0);
      } finally {
        sandbox.cleanup();
      }
    });
  });
});
