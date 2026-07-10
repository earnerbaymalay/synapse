import { describe, it, expect } from 'vitest';
import { createSandbox } from '../helpers/sandbox.js';
import { runCLI } from '../helpers/cli.js';
import { validateData } from '../helpers/schema.js';
import fs from 'node:fs';
import path from 'node:path';

describe('E2E Sanity Tests', () => {
  it('should successfully create and clean up a sandbox', () => {
    const sandbox = createSandbox('sanity-test');
    expect(fs.existsSync(sandbox.tmpDir)).toBe(true);
    expect(fs.existsSync(sandbox.workspaceDir)).toBe(true);
    expect(fs.existsSync(sandbox.brainDir)).toBe(true);
    expect(fs.existsSync(sandbox.mockHome)).toBe(true);

    expect(sandbox.env.HOME).toBe(sandbox.mockHome);
    expect(sandbox.env.NEUROSURGEON_BRAIN_PATH).toBe(sandbox.brainDir);
    expect(sandbox.env.NEUROSURGEON_WORKSPACE_PATH).toBe(sandbox.workspaceDir);

    sandbox.cleanup();
    expect(fs.existsSync(sandbox.tmpDir)).toBe(false);
  });

  it('should run the Rust CLI binary and return output', async () => {
    const sandbox = createSandbox('cli-test');
    try {
      // --help should exit with 0 and print usage instructions
      const result = await runCLI(['--help'], { sandbox });
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toContain('neurosurgeon');
      expect(result.stdout).toContain('scan');
      expect(result.stdout).toContain('import');
    } finally {
      sandbox.cleanup();
    }
  });

  it('should fail running implemented command but capture output', async () => {
    const sandbox = createSandbox('cli-fail-test');
    try {
      // Currently scan is not fully implemented in CLI and exits with Failure
      await expect(runCLI(['scan'], { sandbox })).rejects.toThrow();
    } finally {
      sandbox.cleanup();
    }
  });

  it('should validate canonical schema structure correctly', () => {
    const validSkill = {
      id: 'test-skill',
      version: '1.0.0',
      triggers: ['*.ts'],
      targets: ['cursor'],
      source: 'native',
      sha256: 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6abcd'
    };

    const resValid = validateData(validSkill, 'skill');
    expect(resValid.valid).toBe(true);
    expect(resValid.errors).toBeUndefined();

    const invalidSkill = {
      id: 'test-skill',
      // version is missing
      triggers: ['*.ts'],
      targets: ['cursor'],
      source: 'native',
      sha256: 'invalid-hash' // not 64 chars hex
    };

    const resInvalid = validateData(invalidSkill, 'skill');
    expect(resInvalid.valid).toBe(false);
    expect(resInvalid.errors).toBeDefined();
  });
});
