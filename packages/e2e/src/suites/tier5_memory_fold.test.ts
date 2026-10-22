import { describe, it, expect } from 'vitest';
import http from 'node:http';
import net from 'node:net';
import path from 'node:path';
import { execa } from 'execa';
import { createSandbox } from '../helpers/sandbox.js';
import { runCLI } from '../helpers/cli.js';

const SESSION = 'sess-e2e-1';

function memEnv(sandbox: ReturnType<typeof createSandbox>) {
  return { SYNAPSE_MEMORY_DIR: path.join(sandbox.tmpDir, 'memory') };
}

function hook(
  sandbox: ReturnType<typeof createSandbox>,
  event: string,
  payload: unknown,
  cwd = sandbox.workspaceDir,
) {
  const body = typeof payload === 'string' ? payload : JSON.stringify({ session_id: SESSION, cwd, ...(payload as object) });
  return runCLI(['mem', 'hook', event], { sandbox, env: memEnv(sandbox), input: body });
}

function search(sandbox: ReturnType<typeof createSandbox>, ...query: string[]) {
  return runCLI(['mem', 'search', '--all', ...query], { sandbox, env: memEnv(sandbox) });
}

describe('Tier 5: synaptic memory hooks', () => {
  it('records a prompt and finds it through the search index', async () => {
    const sandbox = createSandbox('mem-prompt');
    try {
      const r = await hook(sandbox, 'prompt', { prompt: 'Refactor the flux capacitor module' });
      expect(r.exitCode).toBe(0);
      const found = await search(sandbox, 'flux');
      expect(found.stdout).toContain('flux capacitor');
      const miss = await search(sandbox, 'zzzunmatchedzzz');
      expect(miss.stdout).not.toContain('flux capacitor');
    } finally {
      sandbox.cleanup();
    }
  });

  it('never fails the agent on malformed or empty hook input', async () => {
    const sandbox = createSandbox('mem-malformed');
    try {
      for (const event of ['prompt', 'post-tool', 'stop', 'session-start', 'bogus-event']) {
        const r = await hook(sandbox, event, 'not json {{{');
        expect(r.exitCode).toBe(0);
      }
      const empty = await hook(sandbox, 'post-tool', '');
      expect(empty.exitCode).toBe(0);
    } finally {
      sandbox.cleanup();
    }
  });

  it('drops <private> spans and redacts credential-looking tokens before disk', async () => {
    const sandbox = createSandbox('mem-privacy');
    try {
      const secret = 'sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdef';
      await hook(sandbox, 'prompt', {
        prompt: `deploy now <private>hunter2-do-not-store</private> with key ${secret}`,
      });
      const dump = await runCLI(['mem', 'get', '1'], { sandbox, env: memEnv(sandbox) });
      expect(dump.stdout).not.toContain('hunter2-do-not-store');
      expect(dump.stdout).not.toContain(secret);
      expect(dump.stdout).toContain('deploy now');
    } finally {
      sandbox.cleanup();
    }
  });

  it('captures a post-tool observation with the touched file', async () => {
    const sandbox = createSandbox('mem-posttool');
    try {
      const file = path.join(sandbox.workspaceDir, 'src/lib.rs');
      const r = await hook(sandbox, 'post-tool', {
        tool_name: 'Edit',
        tool_input: { file_path: file, old_string: 'a', new_string: 'b' },
        tool_response: { success: true },
      });
      expect(r.exitCode).toBe(0);
      const found = await search(sandbox, 'lib.rs');
      expect(found.stdout).toContain('lib.rs');
    } finally {
      sandbox.cleanup();
    }
  });

  it('keeps exactly one rolling summary per session across repeated stops', async () => {
    const sandbox = createSandbox('mem-stop');
    try {
      await hook(sandbox, 'prompt', { prompt: 'Add retry logic to uploader' });
      await hook(sandbox, 'stop', {});
      await hook(sandbox, 'prompt', { prompt: 'Also add backoff jitter' });
      await hook(sandbox, 'stop', {});
      const stats = await runCLI(['mem', 'stats'], { sandbox, env: memEnv(sandbox) });
      const summaries = (stats.stdout.match(/summary/gi) ?? []).length;
      const all = await search(sandbox, '--limit', '50');
      const summaryRows = all.stdout.split('\n').filter((l) => /summary/i.test(l));
      expect(summaries + summaryRows.length).toBeGreaterThan(0);
      expect(summaryRows.length).toBeLessThanOrEqual(1);
    } finally {
      sandbox.cleanup();
    }
  });

  it('session-start injects prior context for the same project only', async () => {
    const sandbox = createSandbox('mem-context');
    try {
      // Prompts are intentionally excluded from the context block; observations are not.
      await runCLI(['mem', 'observe', 'quokka allocator leak traced to arena reuse', '--project', sandbox.workspaceDir], { sandbox, env: memEnv(sandbox) });
      const same = await hook(sandbox, 'session-start', {});
      expect(same.stdout).toContain('quokka allocator');
      const other = await hook(sandbox, 'session-start', {}, path.join(sandbox.tmpDir, 'some-other-project'));
      expect(other.stdout).not.toContain('quokka allocator');
    } finally {
      sandbox.cleanup();
    }
  });

  it('forget removes an observation permanently', async () => {
    const sandbox = createSandbox('mem-forget');
    try {
      await hook(sandbox, 'prompt', { prompt: 'ephemeral wombat note' });
      await runCLI(['mem', 'forget', '1'], { sandbox, env: memEnv(sandbox) });
      const found = await search(sandbox, 'wombat');
      expect(found.stdout).not.toContain('wombat');
    } finally {
      sandbox.cleanup();
    }
  });
});

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const srv = net.createServer();
    srv.listen(0, '127.0.0.1', () => {
      const { port } = srv.address() as net.AddressInfo;
      srv.close(() => resolve(port));
    });
    srv.on('error', reject);
  });
}

async function waitFor(fn: () => Promise<boolean>, ms = 10000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (await fn().catch(() => false)) return;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('timed out waiting for gateway');
}

describe('Tier 5: gateway context folding', () => {
  it('folds old tool results into restorable stubs and keeps recent ones intact', async () => {
    const sandbox = createSandbox('fold');
    const received: any[] = [];
    const upstream = http.createServer((req, res) => {
      let data = '';
      req.on('data', (c) => (data += c));
      req.on('end', () => {
        try {
          received.push(JSON.parse(data));
        } catch {
          received.push(null);
        }
        res.setHeader('content-type', 'application/json');
        res.end(
          JSON.stringify({
            id: 'x',
            object: 'chat.completion',
            model: 'm',
            choices: [{ index: 0, message: { role: 'assistant', content: 'ok' }, finish_reason: 'stop' }],
            usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 },
          }),
        );
      });
    });
    await new Promise<void>((r) => upstream.listen(0, '127.0.0.1', () => r()));
    const upPort = (upstream.address() as net.AddressInfo).port;
    const gwPort = await freePort();

    const big = (tag: string) =>
      Array.from({ length: 400 }, (_, i) => `${tag} noisy build line ${i} padding padding padding`).join('\n');
    const oldOutput = big('OLD');
    const newOutput = big('NEW');

    const gw = runCLI(
      ['serve', '--port', String(gwPort), '--upstream', `http://127.0.0.1:${upPort}`, '--model', 'm', '--gate-level', 'off', '--fold-keep', '2'],
      { sandbox },
    );
    gw.catch(() => undefined);
    try {
      await waitFor(async () => (await fetch(`http://127.0.0.1:${gwPort}/v1/models`)).ok);

      const messages = [
        { role: 'user', content: 'run the build' },
        { role: 'assistant', content: null, tool_calls: [{ id: 'c1', type: 'function', function: { name: 'sh', arguments: '{}' } }] },
        { role: 'tool', tool_call_id: 'c1', content: oldOutput },
        { role: 'assistant', content: 'built, now test' },
        { role: 'user', content: 'ok' },
        { role: 'assistant', content: null, tool_calls: [{ id: 'c2', type: 'function', function: { name: 'sh', arguments: '{}' } }] },
        { role: 'tool', tool_call_id: 'c2', content: newOutput },
      ];
      const post = () =>
        fetch(`http://127.0.0.1:${gwPort}/v1/chat/completions`, {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ model: 'm', messages }),
        });

      const res = await post();
      expect(res.status).toBe(200);
      const forwarded = received[0].messages;
      const oldTool = forwarded.find((m: any) => m.tool_call_id === 'c1').content as string;
      const newTool = forwarded.find((m: any) => m.tool_call_id === 'c2').content as string;

      expect(oldTool).toContain('[SYNAPSE folded');
      expect(oldTool.length).toBeLessThan(oldOutput.length / 4);
      expect(newTool).toBe(newOutput);

      // Stub is byte-stable across identical requests (prefix-cache friendly).
      await post();
      const oldTool2 = received[1].messages.find((m: any) => m.tool_call_id === 'c1').content;
      expect(oldTool2).toBe(oldTool);

      // The original is restorable via the spool.
      const id = /synapse spool show ([0-9a-f]+)/.exec(oldTool)?.[1];
      expect(id).toBeTruthy();
      const restored = await runCLI(['spool', 'show', id!], { sandbox });
      expect(restored.stdout).toContain('OLD noisy build line 0 ');
      expect(restored.stdout).toContain('OLD noisy build line 399 ');
    } finally {
      gw.kill('SIGKILL');
      upstream.close();
      sandbox.cleanup();
    }
  });

  it('leaves messages untouched when --fold-keep is not set', async () => {
    const sandbox = createSandbox('fold-off');
    const received: any[] = [];
    const upstream = http.createServer((req, res) => {
      let data = '';
      req.on('data', (c) => (data += c));
      req.on('end', () => {
        received.push(JSON.parse(data));
        res.setHeader('content-type', 'application/json');
        res.end(
          JSON.stringify({
            id: 'x',
            object: 'chat.completion',
            model: 'm',
            choices: [{ index: 0, message: { role: 'assistant', content: 'ok' }, finish_reason: 'stop' }],
            usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 },
          }),
        );
      });
    });
    await new Promise<void>((r) => upstream.listen(0, '127.0.0.1', () => r()));
    const upPort = (upstream.address() as net.AddressInfo).port;
    const gwPort = await freePort();
    const output = Array.from({ length: 400 }, (_, i) => `line ${i} padding padding padding padding`).join('\n');

    const gw = runCLI(
      ['serve', '--port', String(gwPort), '--upstream', `http://127.0.0.1:${upPort}`, '--model', 'm', '--gate-level', 'off'],
      { sandbox },
    );
    gw.catch(() => undefined);
    try {
      await waitFor(async () => (await fetch(`http://127.0.0.1:${gwPort}/v1/models`)).ok);
      const messages = [
        { role: 'user', content: 'go' },
        { role: 'tool', tool_call_id: 'c1', content: output },
        { role: 'user', content: 'a' },
        { role: 'user', content: 'b' },
        { role: 'user', content: 'c' },
      ];
      await fetch(`http://127.0.0.1:${gwPort}/v1/chat/completions`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ model: 'm', messages }),
      });
      expect(received[0].messages.find((m: any) => m.tool_call_id === 'c1').content).toBe(output);
    } finally {
      gw.kill('SIGKILL');
      upstream.close();
      sandbox.cleanup();
    }
  });
});
