import { defineConfig } from 'vitest/config';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export default defineConfig({
  test: {
    globalSetup: path.resolve(__dirname, './globalSetup.ts'),
    include: ['src/**/*.test.ts'],
    testTimeout: 20000,
  },
});
