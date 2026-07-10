import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

export interface Sandbox {
  tmpDir: string;
  workspaceDir: string;
  brainDir: string;
  mockHome: string;
  logDir: string;
  env: Record<string, string>;
  cleanup: () => void;
}

export function createSandbox(testName: string): Sandbox {
  // Replace unsafe chars in testName for directory naming
  const safeName = testName.replace(/[^a-zA-Z0-9-_]/g, '_');
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), `neuro-e2e-${safeName}-`));
  const workspaceDir = path.join(tmpDir, 'workspace');
  const brainDir = path.join(tmpDir, 'AIBrain');
  const mockHome = path.join(tmpDir, 'home');
  const logDir = path.join(tmpDir, 'logs');

  fs.mkdirSync(workspaceDir, { recursive: true });
  fs.mkdirSync(brainDir, { recursive: true });
  fs.mkdirSync(mockHome, { recursive: true });
  fs.mkdirSync(logDir, { recursive: true });

  const env = {
    HOME: mockHome,
    NEUROSURGEON_BRAIN_PATH: brainDir,
    NEUROSURGEON_WORKSPACE_PATH: workspaceDir,
    NEUROSURGEON_LOG_DIR: logDir,
  };

  const cleanup = () => {
    try {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    } catch (err) {
      console.warn(`Failed to clean up sandbox at ${tmpDir}:`, err);
    }
  };

  return { tmpDir, workspaceDir, brainDir, mockHome, logDir, env, cleanup };
}
