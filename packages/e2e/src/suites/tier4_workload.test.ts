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

describe('Tier 4: Real-world Workloads & End-to-End User Scenarios', () => {
  it('should import legacy cursorrules and convert to modular skills in the brain', async () => {
    const sandbox = createSandbox('workload-legacy-import');
    try {
      writeFile(sandbox.workspaceDir, '.cursorrules', '# Cursor Rules\n\n## Section 1\nRule body 1\n\n## Section 2\nRule body 2\n');

      const result = await runCLI(['import'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);

      // Verify that skills have been modularized
      const skillsDir = path.join(sandbox.brainDir, 'skills');
      expect(fs.existsSync(skillsDir)).toBe(true);
      const skillDirs = fs.readdirSync(skillsDir);
      expect(skillDirs.length).toBeGreaterThan(0);
    } finally {
      sandbox.cleanup();
    }
  });

  it('should project modular rules after deleting the legacy cursorrules file', async () => {
    const sandbox = createSandbox('workload-modular-project');
    try {
      // 1. Set up modular skills in the brain
      const skill1 = {
        id: 'rule-section-1',
        version: '1.0.0',
        triggers: ['*.ts'],
        targets: ['cursor'],
        source: '# Section 1\nRule body 1\n',
        sha256: '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
      };
      const skill2 = {
        id: 'rule-section-2',
        version: '1.0.0',
        triggers: ['*.js'],
        targets: ['cursor'],
        source: '# Section 2\nRule body 2\n',
        sha256: '83cf13426e632b724f7a2d8e404bf7c2d7f8a70c8c9735d461cf6a568c4a93a1'
      };
      writeFile(sandbox.brainDir, 'skills/rule-section-1/skill.yaml', JSON.stringify(skill1));
      writeFile(sandbox.brainDir, 'skills/rule-section-2/skill.yaml', JSON.stringify(skill2));

      // 2. Project
      const result = await runCLI(['project'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);

      // 3. Verify modular files generated
      expect(fs.existsSync(path.join(sandbox.workspaceDir, '.cursor/rules/rule-section-1.mdc'))).toBe(true);
      expect(fs.existsSync(path.join(sandbox.workspaceDir, '.cursor/rules/rule-section-2.mdc'))).toBe(true);
      expect(fs.existsSync(path.join(sandbox.workspaceDir, '.cursorrules'))).toBe(false);
    } finally {
      sandbox.cleanup();
    }
  });

  it('should project a large scale brain with high performance metrics', async () => {
    const sandbox = createSandbox('workload-scaling-perf');
    try {
      // Generate 150 skills, 50 agents, and 30 MCP servers in mock brain
      for (let i = 0; i < 150; i++) {
        const skill = {
          id: `perf-skill-${i}`,
          version: '1.0.0',
          triggers: ['*'],
          targets: ['cursor'],
          source: `# Rule ${i}\nAlways write high performance code.\n`,
          sha256: `sha-${i}`
        };
        writeFile(sandbox.brainDir, `skills/perf-skill-${i}/skill.yaml`, JSON.stringify(skill));
      }
      for (let i = 0; i < 50; i++) {
        const agent = {
          id: `perf-agent-${i}`,
          name: `Agent ${i}`,
          skills: [`perf-skill-${i}`],
          targets: ['cursor']
        };
        writeFile(sandbox.brainDir, `agents/perf-agent-${i}/agent.yaml`, JSON.stringify(agent));
      }
      for (let i = 0; i < 30; i++) {
        const mcp = {
          id: `perf-mcp-${i}`,
          transport: 'stdio',
          command_or_url: 'node',
          env_placeholders: [],
          targets: ['cursor']
        };
        writeFile(sandbox.brainDir, `mcp/perf-mcp-${i}/mcp.yaml`, JSON.stringify(mcp));
      }

      const start = Date.now();
      const result = await runCLI(['project'], { sandbox, reject: false });
      const duration = Date.now() - start;

      expect(result.exitCode).toBe(0);
      expect(duration).toBeLessThan(1000); // Must be fast
    } finally {
      sandbox.cleanup();
    }
  });

  it('should verify all projected files exist in high volume environment', async () => {
    const sandbox = createSandbox('workload-scaling-verify');
    try {
      for (let i = 0; i < 10; i++) {
        const skill = {
          id: `perf-skill-${i}`,
          version: '1.0.0',
          triggers: ['*'],
          targets: ['cursor'],
          source: `# Rule ${i}\nCode rule ${i}\n`,
          sha256: `sha-${i}`
        };
        writeFile(sandbox.brainDir, `skills/perf-skill-${i}/skill.yaml`, JSON.stringify(skill));
      }

      const result = await runCLI(['project'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);

      // Spot check projected files
      for (let i = 0; i < 10; i++) {
        expect(fs.existsSync(path.join(sandbox.workspaceDir, `.cursor/rules/perf-skill-${i}.mdc`))).toBe(true);
      }
    } finally {
      sandbox.cleanup();
    }
  });

  it('should handle Windows CRLF line endings and normalize to LF in brain', async () => {
    const sandbox = createSandbox('workload-normalization-crlf');
    try {
      // Create rules file with CRLF
      const ruleContent = '# Rules\r\nLine 1\r\nLine 2\r\n';
      writeFile(sandbox.workspaceDir, '.clinerules', ruleContent);

      const result = await runCLI(['import'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);

      // Verify stored brain content has LF only
      const skillYamlPath = path.join(sandbox.brainDir, 'skills/cline-rules/skill.yaml');
      expect(fs.existsSync(skillYamlPath)).toBe(true);
      const skill = JSON.parse(fs.readFileSync(skillYamlPath, 'utf8'));
      expect(skill.source).not.toContain('\r\n');
    } finally {
      sandbox.cleanup();
    }
  });

  it('should normalize Windows backslashes in path references to forward slashes in brain configuration', async () => {
    const sandbox = createSandbox('workload-normalization-backslash');
    try {
      // Create config referencing subfolder path using backslashes
      writeFile(sandbox.workspaceDir, '.aider.conf.yml', 'read: ["src\\\\conventions\\\\rules.md"]\n');
      writeFile(sandbox.workspaceDir, 'src/conventions/rules.md', '# Rules\nSpacing rules.\n');

      const result = await runCLI(['import'], { sandbox, reject: false });
      expect(result.exitCode).toBe(0);

      // Verify the path in brain skill yaml is normalized to forward slash
      const skillYamlPath = path.join(sandbox.brainDir, 'skills/rules/skill.yaml');
      expect(fs.existsSync(skillYamlPath)).toBe(true);
      const skill = JSON.parse(fs.readFileSync(skillYamlPath, 'utf8'));
      expect(skill.triggers[0]).not.toContain('\\\\');
      expect(skill.triggers[0]).toContain('/');
    } finally {
      sandbox.cleanup();
    }
  });
});
