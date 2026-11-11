#!/usr/bin/env node
/**
 * Synapse desktop e2e click-through (plain Node + `playwright`, no test runner).
 *
 * Loads the Vite dev server in Chromium with the Tauri backend mocked
 * (src/test/tauri-mock.ts, injected via page.addInitScript), then for every
 * screen: screenshots it, clicks every visible button / link / toggle /
 * select / input, presses every documented shortcut, Tabs through every
 * focusable element checking for a visible focus style, and records console
 * errors/warnings, uncaught exceptions, failed invoke calls, unnamed
 * controls and controls whose click changed nothing observable.
 *
 * Usage (from apps/desktop, dev server already running on 5199):
 *   nice -n 10 node e2e/clickthrough.mjs [--url http://localhost:5199] [--scenario faulty|clean|both]
 *
 * Output: e2e/results/clickthrough.json, e2e/screenshots/*.png, a summary on
 * stdout. Exit code 1 when any bug-class finding was recorded.
 */
import { createRequire } from 'node:module'
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const HERE = dirname(fileURLToPath(import.meta.url))
const APP = resolve(HERE, '..')

function loadPlaywright() {
  try {
    return require('playwright')
  } catch {
    // globally installed (e.g. /opt/npm-tools) — resolve via NODE_PATH-less global root
    const { execSync } = require('node:child_process')
    const root = execSync('npm root -g').toString().trim()
    for (const p of [join(root, 'playwright'), '/opt/npm-tools/node_modules/playwright']) {
      try {
        return require(p)
      } catch {
        /* try next */
      }
    }
    throw new Error('playwright not found')
  }
}
const { chromium } = loadPlaywright()
const esbuild = createRequire(require.resolve('vite', { paths: [APP] }))('esbuild')

const argv = process.argv.slice(2)
const arg = (k, d) => {
  const i = argv.indexOf(k)
  return i >= 0 ? argv[i + 1] : d
}
const URL = arg('--url', 'http://localhost:5199')
const SCEN = arg('--scenario', 'both')
const SHOTS = join(HERE, 'screenshots')
const OUT = join(HERE, 'results')
mkdirSync(SHOTS, { recursive: true })
mkdirSync(OUT, { recursive: true })

const mockJs = esbuild.transformSync(readFileSync(join(APP, 'src/test/tauri-mock.ts'), 'utf8'), {
  loader: 'ts',
  format: 'iife',
  globalName: '__TauriMockModule',
}).code

const SCREENS = [
  { id: 'intake', key: '1', label: 'Intake' },
  { id: 'examination', key: '2', label: 'Examination' },
  { id: 'cockpit', key: '3', label: 'Cockpit' },
  { id: 'memory', key: '4', label: 'Memory' },
  { id: 'spool', key: '5', label: 'Sensory Spool' },
]

const CLICKABLE =
  'button, a[href], [role="button"], [role="link"], [role="switch"], [role="tab"], [role="checkbox"], [role="menuitem"], input, select, textarea, summary, [onclick]'
const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"]), summary, [contenteditable="true"]'

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

/** In-page helpers, installed once per page. */
const PAGE_HELPERS = () => {
  const visible = (el) => {
    const r = el.getBoundingClientRect()
    const s = getComputedStyle(el)
    return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none' && +s.opacity !== 0
  }
  const text = (el) => (el.innerText ?? el.textContent ?? '').replace(/\s+/g, ' ').trim()
  const accName = (el) => {
    const lb = el.getAttribute('aria-labelledby')
    if (lb)
      return lb
        .split(/\s+/)
        .map((id) => document.getElementById(id))
        .filter(Boolean)
        .map(text)
        .join(' ')
        .trim()
    const al = el.getAttribute('aria-label')
    if (al && al.trim()) return al.trim()
    if (el.id) {
      const l = document.querySelector(`label[for="${CSS.escape(el.id)}"]`)
      if (l && text(l)) return text(l)
    }
    const wrap = el.closest('label')
    if (wrap && text(wrap)) return text(wrap)
    if (el.tagName === 'SELECT') {
      return el.getAttribute('title') || ''
    }
    if (el.tagName !== 'INPUT' && el.tagName !== 'TEXTAREA') {
      // name from content, excluding aria-hidden descendants
      const clone = el.cloneNode(true)
      clone.querySelectorAll('[aria-hidden="true"],[aria-hidden=""]').forEach((n) => n.remove())
      const t = (clone.textContent || '').replace(/\s+/g, ' ').trim()
      if (t) return t
      const img = el.querySelector('img[alt],svg[aria-label]')
      if (img) return img.getAttribute('alt') || img.getAttribute('aria-label') || ''
    }
    return el.getAttribute('title') || ''
  }
  const describe = (el) => ({
    tag: el.tagName.toLowerCase(),
    type: el.getAttribute('type'),
    role: el.getAttribute('role'),
    name: accName(el),
    text: text(el).slice(0, 80),
    placeholder: el.getAttribute('placeholder'),
    disabled: !!el.disabled || el.getAttribute('aria-disabled') === 'true',
    alreadyOn: el.getAttribute('aria-current') === 'page' || el.getAttribute('aria-pressed') === 'true' || el.getAttribute('aria-selected') === 'true',
    inDialog: !!el.closest('[role="dialog"],dialog'),
    region: el.closest('header,[data-tauri-drag-region]')
      ? 'titlebar'
      : el.closest('aside,nav')
        ? 'sidebar'
        : el.closest('main')
          ? 'main'
          : 'other',
  })
  /** digit-normalised structural signature of the DOM (ignores animated counters). */
  const signature = () => {
    const out = []
    for (const el of document.body.querySelectorAll('*')) {
      if (el.tagName === 'CANVAS' || el.tagName === 'SCRIPT' || el.tagName === 'STYLE') continue
      if (!visible(el)) continue
      let own = ''
      for (const n of el.childNodes) if (n.nodeType === 3) own += n.nodeValue
      const attrs = ['class', 'aria-pressed', 'aria-expanded', 'aria-current', 'aria-checked', 'aria-selected', 'disabled', 'open', 'value', 'style']
        .map((a) => (a === 'style' ? '' : el.getAttribute(a) ?? ''))
        .join('|')
      out.push(`${el.tagName}|${attrs}|${own.trim()}`.replace(/\d+(\.\d+)?/g, '#'))
    }
    return out
  }
  window.__E2E__ = { visible, accName, describe, signature, text }
}

function diffCount(a, b) {
  const m = new Map()
  for (const s of a) m.set(s, (m.get(s) ?? 0) + 1)
  let d = 0
  for (const s of b) {
    const c = m.get(s) ?? 0
    if (c > 0) m.set(s, c - 1)
    else d++
  }
  for (const c of m.values()) d += c
  return d
}

async function state(page) {
  return page.evaluate(() => ({
    sig: window.__E2E__.signature(),
    calls: window.__TAURI_MOCK__?.calls.length ?? 0,
    win: JSON.stringify(window.__TAURI_MOCK__?.window ?? {}),
    ls: JSON.stringify({ ...localStorage }),
    active: document.activeElement ? window.__E2E__.describe(document.activeElement).name : '',
    screen: document.querySelector('[aria-current="page"]')?.innerText?.split('\n')[0] ?? '',
    dialog: !!document.querySelector('[role="dialog"],dialog[open]'),
  }))
}

function changed(before, after) {
  const why = []
  const d = diffCount(before.sig, after.sig)
  if (d > 0) why.push(`dom(${d})`)
  if (after.calls !== before.calls) why.push(`invoke(+${after.calls - before.calls})`)
  if (after.win !== before.win) why.push('window-state')
  if (after.ls !== before.ls) why.push('localStorage')
  if (after.screen !== before.screen) why.push(`screen->${after.screen}`)
  if (after.dialog !== before.dialog) why.push(after.dialog ? 'dialog-open' : 'dialog-closed')
  return why
}

async function currentScreen(page) {
  return page.evaluate(() => document.querySelector('[aria-current="page"]')?.innerText?.split('\n')[0] ?? '')
}

async function goTo(page, s) {
  const nav = page.locator('nav button', { hasText: s.label }).first()
  if (await nav.count()) await nav.click()
  else await page.keyboard.press(`Control+${s.key}`)
  await sleep(500)
}

async function closeOverlays(page) {
  for (let i = 0; i < 3; i++) {
    const open = await page.evaluate(() => !!document.querySelector('[role="dialog"],dialog[open]'))
    if (!open) return
    await page.keyboard.press('Escape')
    await sleep(200)
  }
}

async function listControls(page) {
  return page.evaluate((SEL) => {
    const els = [...document.querySelectorAll(SEL)].filter((e) => window.__E2E__.visible(e))
    return els.map((e, i) => ({ i, ...window.__E2E__.describe(e) }))
  }, CLICKABLE)
}

/** Click one control (re-found by its descriptor); returns observable effect. */
async function exercise(page, ctl, scr, rec) {
  const navBefore = rec.navigations.length
  try {
    const r = await exerciseInner(page, ctl, scr, rec)
    if (rec.navigations.length > navBefore) r.navigated = rec.navigations.slice(navBefore)
    return r
  } catch (e) {
    const msg = String(e).split('\n')[0]
    if (rec.navigations.length > navBefore || /context was destroyed|navigation/i.test(msg)) {
      await page.waitForLoadState('networkidle').catch(() => {})
      await sleep(1000)
      return { navigated: rec.navigations.slice(navBefore), failed: `page navigated/reloaded: ${msg}` }
    }
    return { failed: msg }
  }
}

async function exerciseInner(page, ctl, scr, rec) {
  const handle = await page.evaluateHandle(
    ({ SEL, ctl }) => {
      const els = [...document.querySelectorAll(SEL)].filter((e) => window.__E2E__.visible(e))
      return (
        els.find((e) => {
          const d = window.__E2E__.describe(e)
          return d.tag === ctl.tag && d.name === ctl.name && d.text === ctl.text && d.region === ctl.region
        }) ?? null
      )
    },
    { SEL: CLICKABLE, ctl },
  )
  const el = handle.asElement()
  if (!el) return { skipped: 'not found after re-render' }
  if (ctl.disabled) return { skipped: 'disabled' }
  const before = await state(page)
  const errBefore = rec.errors.length
  try {
    if (ctl.tag === 'select') {
      const opts = await el.evaluate((s) => [...s.options].map((o) => o.value))
      const cur = await el.evaluate((s) => s.value)
      const next = opts.find((o) => o !== cur) ?? cur
      await el.selectOption(next)
    } else if (ctl.tag === 'input' && !['checkbox', 'radio', 'button', 'submit'].includes(ctl.type ?? 'text')) {
      await el.fill('obs')
      await sleep(300)
      const mid = await state(page)
      await el.fill('')
      await page.mouse.click(2, 300)
      const why = changed(before, mid)
      return { effect: why, errors: rec.errors.slice(errBefore) }
    } else {
      await el.click({ timeout: 2000 })
    }
  } catch (e) {
    const lines = String(e).split('\n')
    const why = lines.find((l) => /intercepts pointer events|not visible|outside of the viewport|detached/.test(l))
    return { failed: lines[0] + (why ? ` — ${why.trim()}` : '') }
  }
  await sleep(450)
  const after = await state(page)
  return { effect: changed(before, after), errors: rec.errors.slice(errBefore) }
}

async function focusAudit(page) {
  await page.mouse.move(1, 1)
  await page.evaluate(() => {
    document.activeElement?.blur?.()
    const STYLE_KEYS = ['outlineStyle', 'outlineWidth', 'outlineColor', 'boxShadow', 'borderColor', 'backgroundColor', 'color', 'textDecorationLine']
    window.__E2E__.styleOf = (el) => {
      const s = getComputedStyle(el)
      return STYLE_KEYS.map((k) => s[k]).join(';')
    }
  })
  const baseline = await page.evaluate((SEL) => {
    const m = []
    for (const el of document.querySelectorAll(SEL)) {
      if (!window.__E2E__.visible(el)) continue
      el.dataset.e2eFocus = String(m.length)
      m.push(window.__E2E__.styleOf(el))
    }
    return m
  }, FOCUSABLE)
  const seen = []
  const noStyle = []
  let nullStops = 0
  // start from the top of the document
  await page.evaluate(() => {
    document.body.setAttribute('tabindex', '-1')
    document.body.focus()
    document.body.removeAttribute('tabindex')
  })
  for (let i = 0; i < 80; i++) {
    await page.keyboard.press('Tab')
    const r = await page.evaluate(() => {
      const el = document.activeElement
      if (!el || el === document.body) return null
      const idx = el.dataset.e2eFocus
      return {
        idx: idx ?? null,
        d: window.__E2E__.describe(el),
        style: window.__E2E__.styleOf(el),
        fv: el.matches(':focus-visible'),
        outline: getComputedStyle(el).outlineStyle,
      }
    })
    if (!r) {
      nullStops++
      if (nullStops > 3 || seen.length) break
      continue
    }
    const key = `${r.idx}|${r.d.name}|${r.d.tag}`
    if (seen.includes(key)) break
    seen.push(key)
    const base = r.idx != null ? baseline[+r.idx] : null
    const visibleFocus = base == null ? r.outline !== 'none' : base !== r.style
    if (!visibleFocus) noStyle.push(`${r.d.tag} "${r.d.name || r.d.placeholder || r.d.text}" (${r.d.region})`)
  }
  await page.evaluate(() => document.activeElement?.blur?.())
  return { tabStops: seen.length, noVisibleFocus: noStyle }
}

async function shortcuts(page, rec) {
  const res = []
  const press = async (combo, check) => {
    const before = await state(page)
    const errB = rec.errors.length
    await page.keyboard.press(combo)
    await sleep(450)
    const after = await state(page)
    const effect = changed(before, after)
    const verdict = check(before, after, effect)
    res.push({ combo, effect, pass: verdict === true, note: verdict === true ? '' : verdict, errors: rec.errors.slice(errB) })
  }
  for (const s of SCREENS) {
    await press(`Control+${s.key}`, (b, a) => (a.screen === s.label ? true : `screen stayed "${a.screen}"`))
  }
  await goTo(page, SCREENS[0])
  await press('Control+r', (b, a) => {
    if (a.calls <= b.calls) return 'no invoke issued'
    return true
  })
  const reload = await page.evaluate(() => performance.getEntriesByType('navigation').length)
  await press('Control+h', (b, a) => (a.ls !== b.ls || diffCount(b.sig, a.sig) > 0 ? true : 'hints state unchanged'))
  await press('Control+h', (b, a) => (a.ls !== b.ls || diffCount(b.sig, a.sig) > 0 ? true : 'hints state unchanged'))
  // screen-scoped bare-key shortcuts (registered in the screens)
  const scoped = [
    ['intake', 'r', (b, a) => (a.calls > b.calls ? true : 'no invoke (intake re-scan)')],
    ['cockpit', 'p', (b, a, e) => (e.length ? true : 'nothing observable')],
    ['memory', 'l', (b, a, e) => (e.length ? true : 'nothing observable (may be gated on WebGL ok)')],
    ['memory', 'l', (b, a, e) => (e.length ? true : 'nothing observable')],
    ['memory', '0', (b, a, e) => (true)],
    ['spool', 'v', (b, a, e) => (e.length ? true : 'view did not change')],
    ['spool', 'v', (b, a, e) => (e.length ? true : 'view did not change')],
    ['spool', 'v', (b, a, e) => (e.length ? true : 'view did not change')],
    ['spool', 'Shift+R', (b, a) => (a.calls > b.calls ? true : 'no spool_list re-read')],
  ]
  for (const [id, key, check] of scoped) {
    const s = SCREENS.find((x) => x.id === id)
    if ((await currentScreen(page)) !== s.label) await goTo(page, s)
    await press(key, check)
    res[res.length - 1].combo = `${id}:${key}`
  }
  await goTo(page, SCREENS[0])
  for (const k of ['Shift+?', 'Control+/', 'Control+k']) {
    await press(k, (b, a) => (a.dialog ? true : 'no dialog opened'))
    await press('Escape', (b, a) => (!a.dialog ? true : 'dialog still open'))
  }
  // `?` must not fire while typing
  const input = page.locator('input[type="text"],input:not([type]),textarea').first()
  if (await input.count()) {
    await input.focus()
    await press('Shift+?', (b, a) => (!a.dialog ? true : 'dialog opened while typing'))
    await page.keyboard.press('Escape')
  }
  return { shortcuts: res, navigationEntries: reload }
}

async function run(scenario) {
  const browser = await chromium.launch()
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, permissions: ['clipboard-read', 'clipboard-write'] })
  const page = await context.newPage()
  const rec = { errors: [], console: [], navigations: [] }
  page.on('framenavigated', (f) => {
    if (f === page.mainFrame()) rec.navigations.push({ url: f.url(), t: Date.now() })
  })
  page.on('console', (m) => {
    if (m.type() === 'error' || m.type() === 'warning') {
      const e = { type: m.type(), text: m.text().slice(0, 400), loc: m.location()?.url?.replace(URL, '') + ':' + m.location()?.lineNumber }
      rec.console.push(e)
      rec.errors.push(e)
    }
  })
  page.on('pageerror', (e) => {
    const x = { type: 'pageerror', text: String(e.stack || e).slice(0, 600) }
    rec.console.push(x)
    rec.errors.push(x)
  })
  await page.addInitScript({
    content: `${mockJs}\n__TauriMockModule.installTauriMock(window, ${JSON.stringify({ scenario })});`,
  })
  await page.addInitScript(PAGE_HELPERS)
  await page.goto(URL, { waitUntil: 'networkidle' })
  await sleep(1200)

  const report = { scenario, screens: {}, global: {} }
  report.global.boot = await page.evaluate(() => ({
    title: document.title,
    calls: window.__TAURI_MOCK__.calls.map((c) => c.cmd),
  }))

  for (const s of SCREENS) {
    await closeOverlays(page)
    await goTo(page, s)
    const scr = { label: s.label, controls: [], unnamed: [], noop: [], failed: [] }
    await page.screenshot({ path: join(SHOTS, `${scenario}-${s.id}.png`) })
    const controls = await listControls(page)
    for (const c of controls) {
      if (!c.name && !(c.tag === 'input' && c.placeholder)) scr.unnamed.push(`${c.tag}${c.type ? `[${c.type}]` : ''} text="${c.text}" (${c.region})`)
      else if (!c.name) scr.unnamed.push(`${c.tag} placeholder-only "${c.placeholder}" (${c.region})`)
    }
    if (s.id === 'memory') {
      const box = await page.locator('canvas').first().boundingBox().catch(() => null)
      scr.canvas = !!box
      if (box) {
        const before = await state(page)
        for (const [fx, fy] of [[0.5, 0.5], [0.45, 0.5], [0.55, 0.45], [0.5, 0.55], [0.4, 0.4], [0.6, 0.6]]) {
          await page.mouse.click(box.x + box.width * fx, box.y + box.height * fy)
          await sleep(350)
        }
        scr.canvasClickEffect = changed(before, await state(page))
        await page.screenshot({ path: join(SHOTS, `${scenario}-memory-canvas-click.png`) })
        await page.keyboard.press('Escape')
        await sleep(300)
      }
    }
    // the shell's controls are exercised once (on the first screen), screen controls on every screen
    const LAST = /^(list view|graph view|hints o(n|ff)|turn hints off|got it|hide)/i
    const todo = controls
      .filter((c) => (s.id === 'intake' ? true : c.region === 'main' || c.inDialog))
      .sort((a, b) => Number(LAST.test(a.name)) - Number(LAST.test(b.name)))
    for (const c of todo) {
      // nav buttons move us away; exercise and come back
      const r = await exercise(page, c, s, rec)
      const label = `${c.tag} "${c.name || c.placeholder || c.text}" (${c.region})`
      if (r.failed) scr.failed.push(`${label}: ${r.failed}`)
      else if (!r.skipped && r.effect && r.effect.length === 0) {
        if (c.alreadyOn) r.expectedNoop = 'already current/pressed'
        else if (s.id === 'memory' && /reset view/i.test(c.name)) r.expectedNoop = 'canvas-only effect (camera), not DOM-observable'
        else scr.noop.push(label)
      }
      scr.controls.push({ label, ...r })
      await closeOverlays(page)
      if ((await currentScreen(page)) !== s.label) await goTo(page, s)
    }
    // second pass: controls that only appeared after the first pass (list view rows, drawers, expanded findings)
    const seenNames = new Set(controls.map((c) => `${c.tag}|${c.name}`))
    const fresh = (await listControls(page)).filter(
      (c) => !seenNames.has(`${c.tag}|${c.name}`) && (c.region === 'main' || c.inDialog),
    )
    fresh.sort((a, b) => Number(LAST.test(a.name)) - Number(LAST.test(b.name)))
    for (const c of fresh.slice(0, 25)) {
      if (!c.name && !c.placeholder) scr.unnamed.push(`${c.tag} text="${c.text}" (${c.region}, second pass)`)
      const r = await exercise(page, c, s, rec)
      const label = `${c.tag} "${c.name || c.placeholder || c.text}" (${c.region}, 2nd pass)`
      if (r.failed) scr.failed.push(`${label}: ${r.failed}`)
      else if (!r.skipped && r.effect && r.effect.length === 0) {
        if (c.alreadyOn) r.expectedNoop = 'already current/pressed'
        else scr.noop.push(label)
      }
      scr.controls.push({ label, ...r })
      await closeOverlays(page)
      if ((await currentScreen(page)) !== s.label) await goTo(page, s)
    }
    scr.focus = await focusAudit(page)
    await page.screenshot({ path: join(SHOTS, `${scenario}-${s.id}-after.png`) })
    report.screens[s.id] = scr
  }

  // title bar window controls, asserted against the mock
  await closeOverlays(page)
  report.global.windowControls = await page.evaluate(() =>
    window.__TAURI_MOCK__.calls.filter((c) => c.cmd.startsWith('plugin:window|')).map((c) => c.cmd),
  )

  report.global.keyboard = await shortcuts(page, rec)
  // command reference screenshot
  await page.keyboard.press('Shift+?')
  await sleep(400)
  await page.screenshot({ path: join(SHOTS, `${scenario}-command-reference.png`) })
  const dialogInfo = await page.evaluate(() => {
    const d = document.querySelector('[role="dialog"],dialog[open]')
    if (!d) return null
    return {
      name: d.getAttribute('aria-label') || (d.getAttribute('aria-labelledby') && document.getElementById(d.getAttribute('aria-labelledby'))?.innerText) || '',
      modal: d.getAttribute('aria-modal'),
      focusInside: d.contains(document.activeElement),
    }
  })
  report.global.commandReference = dialogInfo
  if (dialogInfo) {
    const inside = []
    for (let i = 0; i < 15; i++) {
      await page.keyboard.press('Tab')
      inside.push(await page.evaluate(() => !!document.activeElement?.closest('[role="dialog"],dialog')))
    }
    report.global.commandReference.focusTrapped = inside.every(Boolean)
    await page.keyboard.press('Escape')
    await sleep(300)
    report.global.commandReference.closedByEsc = !(await page.evaluate(() => !!document.querySelector('[role="dialog"],dialog[open]')))
  }

  report.global.mock = await page.evaluate(() => ({
    failures: window.__TAURI_MOCK__.failures,
    callCounts: window.__TAURI_MOCK__.calls.reduce((m, c) => ((m[c.cmd] = (m[c.cmd] ?? 0) + 1), m), {}),
    window: window.__TAURI_MOCK__.window,
  }))
  report.console = rec.console
  report.navigations = rec.navigations
  await browser.close()
  return report
}

const scenarios = SCEN === 'both' ? ['faulty', 'clean'] : [SCEN]
const all = []
for (const s of scenarios) all.push(await run(s))
writeFileSync(join(OUT, 'clickthrough.json'), JSON.stringify(all, null, 2))

let bugs = 0
for (const r of all) {
  console.log(`\n=== scenario: ${r.scenario} === boot invokes: ${r.global.boot.calls.join(', ')}`)
  for (const [id, s] of Object.entries(r.screens)) {
    const ok = s.controls.filter((c) => c.effect?.length).length
    console.log(`\n[${id}] controls exercised: ${s.controls.length}, with effect: ${ok}, tab stops: ${s.focus.tabStops}`)
    for (const c of s.controls) console.log(`   ${c.effect?.length ? 'ok  ' : c.skipped ? 'skip' : c.expectedNoop ? 'ok* ' : c.failed ? 'FAIL' : 'NOOP'} ${c.label} ${c.effect ? c.effect.join(',') : c.skipped ?? c.failed ?? ''}`)
    if (s.noop.length) console.log(`   no observable effect: ${s.noop.join(' ; ')}`)
    if (s.failed.length) console.log(`   click failed: ${s.failed.join(' ; ')}`)
    if (s.unnamed.length) console.log(`   no accessible name: ${s.unnamed.join(' ; ')}`)
    if (s.focus.noVisibleFocus.length) console.log(`   no visible focus: ${s.focus.noVisibleFocus.join(' ; ')}`)
    if (s.canvas !== undefined) console.log(`   canvas present: ${s.canvas}; canvas clicks changed: ${JSON.stringify(s.canvasClickEffect ?? null)}`)
    bugs += s.noop.length + s.failed.length
  }
  console.log('\n[keyboard]')
  for (const k of r.global.keyboard.shortcuts) console.log(`   ${k.pass ? 'pass' : 'FAIL'} ${k.combo} ${k.effect.join(',')} ${k.note}`)
  bugs += r.global.keyboard.shortcuts.filter((k) => !k.pass).length
  console.log(`   command reference: ${JSON.stringify(r.global.commandReference)}`)
  console.log(`   window-control invokes: ${r.global.windowControls.join(', ') || '(none)'}`)
  console.log(`   mock failures: ${JSON.stringify(r.global.mock.failures)}`)
  console.log(`   invoke counts: ${JSON.stringify(r.global.mock.callCounts)}`)
  const uniq = [...new Map(r.console.map((c) => [c.type + c.text.slice(0, 160), c])).values()]
  console.log(`   console (${r.console.length} total, ${uniq.length} unique):`)
  for (const c of uniq) console.log(`     ${c.type}: ${c.text.slice(0, 240).replace(/\n/g, ' | ')} ${c.loc ?? ''}`)
  console.log(`   main-frame navigations (1 = initial load): ${r.navigations.length}`)
  bugs += r.global.mock.failures.length + r.console.filter((c) => c.type !== 'warning').length
}
console.log(`\nresults: ${join(OUT, 'clickthrough.json')}  screenshots: ${SHOTS}`)
process.exit(bugs ? 1 : 0)
