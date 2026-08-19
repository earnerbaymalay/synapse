/** @type {import('tailwindcss').Config} */
// SYNAPSE tokens (brands/synapse/tokens.json), exposed as CSS variables in
// index.css. `drape` is the one working UI accent (Synapse Blue). Gold is
// deliberately absent here — the brand book reserves it for the marketing
// hero device only, never UI chrome, so it has no Tailwind color at all in
// the app.
export default {
  darkMode: "media",
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        ground: "var(--ground)",
        chart: "var(--chart)",
        rule: "var(--rule)",
        ink: "var(--ink)",
        "ink-soft": "var(--ink-soft)",
        drape: "var(--drape)",
        "drape-light": "var(--drape-light)",
        success: "var(--success)",
        alarm: "var(--alarm)",
        caution: "var(--caution)",
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
