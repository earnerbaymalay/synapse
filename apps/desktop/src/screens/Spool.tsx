import { useState } from 'react'

interface SpooledRun {
  id: string
  command: string
  timestamp: string
  rawTokens: number
  gatedTokens: number
  reduction: number
  rawSample: string
  gatedSample: string
}

const SAMPLE_SPOOLS: SpooledRun[] = [
  {
    id: 'spool-9481',
    command: 'cargo test --workspace',
    timestamp: '2 mins ago',
    rawTokens: 8420,
    gatedTokens: 240,
    reduction: 97.1,
    rawSample: `   Compiling synapse-core v1.0.0 (/workspace/synapse/packages/core)
    Finished \`test\` profile in 14.8s
     Running unittests src/lib.rs (target/debug/deps/synapse_core-9e231d4004220db7)
test adapters::agy_cli::tests::test_agy_cli_detect ... ok
test adapters::aider::tests::test_aider_detect ... ok
test adapters::claude_code::tests::test_claude_code_detect ... ok
test adapters::cline::tests::test_cline_detect ... ok
test adapters::continue_adapter::tests::test_continue_detect ... ok
[... 230 passing lines truncated ...]
test sync::tests::perform_import_and_project_roundtrip ... ok
test result: ok. 237 passed; 0 failed; 0 ignored; 0 measured; finished in 15.4s`,
    gatedSample: `[SYNAPSE SENSORY GATE] Output filtered (Aggressive mode)
✔ cargo test --workspace: 237/237 passed in 15.4s (0 failed, 0 warnings)
✔ Preserved 100% of assertion traces and diagnostics.
[Raw spooled to ~/.synapse/spool/spool-9481.log]`,
  },
  {
    id: 'spool-9480',
    command: 'pnpm --filter @synapse/e2e test',
    timestamp: '14 mins ago',
    rawTokens: 6840,
    gatedTokens: 210,
    reduction: 96.9,
    rawSample: ` RUN  v4.1.12 /workspace/synapse/packages/e2e
 ✓ src/suites/sanity.test.ts (4 tests) 53ms
 ✓ src/suites/tier4_workload.test.ts (6 tests) 400ms
 ✓ src/suites/tier2_boundary.test.ts (68 tests) 1272ms
 ✓ src/suites/tier3_combination.test.ts (12 tests) 1424ms
 ✓ src/suites/tier1_feature.test.ts (70 tests) 2908ms
 Test Files  5 passed (5)
      Tests  160 passed (160)`,
    gatedSample: `[SYNAPSE SENSORY GATE] Output filtered
✔ vitest run: 160/160 tests passed across 5 suites in 5.3s
[Raw spooled to ~/.synapse/spool/spool-9480.log]`,
  },
]

export function Spool() {
  const [selectedRun, setSelectedRun] = useState<SpooledRun>(SAMPLE_SPOOLS[0])
  const [viewMode, setViewMode] = useState<'side-by-side' | 'gated' | 'raw'>('side-by-side')

  return (
    <div className="space-y-6">
      <div className="border-b border-rule pb-3">
        <h2 className="font-display text-base tracking-wider uppercase text-drape">
          Sensory Gate // Spool & Compression Inspector
        </h2>
        <p className="font-mono text-xs text-ink-soft">
          Inspect spooled execution logs and audit token compression fidelity in real time.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {/* Left: Spool List */}
        <div className="border border-rule bg-ground p-3 space-y-2">
          <p className="font-mono text-[11px] font-bold uppercase tracking-wider text-drape px-2 pb-1 border-b border-rule">
            Recent Spooled Executions
          </p>
          <div className="space-y-1">
            {SAMPLE_SPOOLS.map((run) => (
              <button
                key={run.id}
                onClick={() => setSelectedRun(run)}
                className={`w-full text-left p-2.5 font-mono text-xs border transition ${
                  selectedRun.id === run.id
                    ? 'border-drape bg-chart text-ink'
                    : 'border-transparent text-ink-soft hover:text-ink'
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="font-semibold text-drape truncate max-w-[150px]">
                    {run.command}
                  </span>
                  <span className="text-semantic-success text-[11px]">
                    -{run.reduction}%
                  </span>
                </div>
                <div className="flex items-center justify-between text-[11px] text-ink-soft mt-1">
                  <span>{run.id}</span>
                  <span>{run.timestamp}</span>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Right: Inspection & Comparison */}
        <div className="md:col-span-2 border border-rule bg-chart p-4 space-y-4">
          <div className="flex flex-wrap items-center justify-between border-b border-rule pb-3">
            <div>
              <h3 className="font-mono text-sm font-semibold text-ink">
                {selectedRun.command}
              </h3>
              <p className="font-mono text-[11px] text-ink-soft">
                {selectedRun.id} · Raw: {selectedRun.rawTokens} tokens ➔ Gated: {selectedRun.gatedTokens} tokens ({selectedRun.reduction}% saved)
              </p>
            </div>

            <div className="flex items-center gap-1">
              <button
                onClick={() => setViewMode('side-by-side')}
                className={`px-2.5 py-1 font-mono text-xs border transition ${
                  viewMode === 'side-by-side' ? 'border-drape bg-drape/10 text-drape' : 'border-rule text-ink-soft'
                }`}
              >
                Side-by-Side
              </button>
              <button
                onClick={() => setViewMode('gated')}
                className={`px-2.5 py-1 font-mono text-xs border transition ${
                  viewMode === 'gated' ? 'border-drape bg-drape/10 text-drape' : 'border-rule text-ink-soft'
                }`}
              >
                Gated View
              </button>
              <button
                onClick={() => setViewMode('raw')}
                className={`px-2.5 py-1 font-mono text-xs border transition ${
                  viewMode === 'raw' ? 'border-drape bg-drape/10 text-drape' : 'border-rule text-ink-soft'
                }`}
              >
                Raw View
              </button>
            </div>
          </div>

          {/* Diff Display */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {(viewMode === 'side-by-side' || viewMode === 'raw') && (
              <div className="border border-rule bg-ground p-3">
                <span className="font-mono text-[11px] font-bold text-semantic-error uppercase">
                  Raw Stderr/Stdout ({selectedRun.rawTokens} tokens)
                </span>
                <pre className="mt-2 overflow-x-auto font-mono text-[11px] leading-relaxed text-ink-soft max-h-80">
                  {selectedRun.rawSample}
                </pre>
              </div>
            )}

            {(viewMode === 'side-by-side' || viewMode === 'gated') && (
              <div className="border border-semantic-success/30 bg-ground p-3">
                <span className="font-mono text-[11px] font-bold text-semantic-success uppercase">
                  Sensory Gated Context ({selectedRun.gatedTokens} tokens)
                </span>
                <pre className="mt-2 overflow-x-auto font-mono text-[11px] leading-relaxed text-semantic-success max-h-80">
                  {selectedRun.gatedSample}
                </pre>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
