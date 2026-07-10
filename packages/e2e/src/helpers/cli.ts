import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execa, type ExecaChildProcess, type Options } from 'execa';
import type { Sandbox } from './sandbox.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Binary will be at workspace_root/target/debug/neurosurgeon
const BINARY_PATH = path.resolve(__dirname, '../../../../target/debug/neurosurgeon');

export interface RunCLIOptions extends Options {
  sandbox?: Sandbox;
}

export function runCLI(args: string[], options?: RunCLIOptions): ExecaChildProcess {
  const mergedEnv = {
    ...process.env,
    ...(options?.sandbox?.env || {}),
    ...options?.env,
  };

  return execa(BINARY_PATH, args, {
    ...options,
    env: mergedEnv,
  });
}
