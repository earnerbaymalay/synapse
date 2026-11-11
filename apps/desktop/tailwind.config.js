/** @type {import('tailwindcss').Config} */
// SYNAPSE tokens (brands/synapse/tokens.json), exposed as CSS variables in
// index.css. `drape` is the one working UI accent (Synapse Blue). Gold is
// deliberately absent here — the brand book reserves it for the marketing
// hero device only, never UI chrome, so it has no Tailwind color at all in
// the app.
// Tokens are plain hex variables, so Tailwind cannot derive `border-rule/60`
// and friends on its own (it silently emitted no CSS for them and borders fell
// back to the preflight light grey). color-mix applies the opacity modifier
// while keeping the variables as ordinary colours for hand-written CSS.
const token = (name) => `color-mix(in srgb, var(${name}) calc(<alpha-value> * 100%), transparent)`

export default {
  darkMode: "media",
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        ground: token("--ground"),
        chart: token("--chart"),
        rule: token("--rule"),
        ink: token("--ink"),
        "ink-soft": token("--ink-soft"),
        drape: token("--drape"),
        "drape-light": token("--drape-light"),
        success: token("--success"),
        alarm: token("--alarm"),
        caution: token("--caution"),
      },
      fontFamily: {
        // UI — all interface text.
        sans: ["Inter", "-apple-system", "Segoe UI", "Roboto", "sans-serif"],
        // Technical — commands, code, paths, eyebrows, status readouts. A
        // chart is a table; columns must align.
        mono: [
          "JetBrains Mono",
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "monospace",
        ],
        // Display — wordmark, hero headlines, stat callouts. Uppercase,
        // sparing use only (brand book: "never a fourth font").
        display: ["Rubik Mono One", "ui-monospace", "monospace"],
      },
      borderRadius: {
        // Square corners everywhere except the pendant/chain marketing
        // device — deliberate, not an oversight (tokens.json `radius`).
        none: "0",
      },
    },
  },
  plugins: [],
}
