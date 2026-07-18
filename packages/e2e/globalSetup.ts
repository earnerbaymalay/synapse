import { execa } from 'execa';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export default async function setup() {
  console.log('Building neurosurgeon CLI...');
  const projectRoot = path.resolve(__dirname, '../..');
  
  const homeDir = os.homedir();
  const cargoBin = path.join(homeDir, '.cargo/bin');
  const env = {
    ...process.env,
    PATH: `${cargoBin}${path.delimiter}${process.env.PATH || ''}`,
  };

  await execa('cargo', ['build', '--bin', 'neurosurgeon'], {
    cwd: projectRoot,
    stdio: 'inherit',
    env,
  });
  console.log('neurosurgeon CLI built successfully.');
}
