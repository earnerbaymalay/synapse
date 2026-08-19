/** @type {import('tailwindcss').Config} */
// Tokens are the IDENTITY.md palette ("The Operating Theatre"), exposed as
// CSS variables in index.css so light and dark resolve without duplicating
// every utility. Deliberately absent: a `primary` ramp — the identity allows
// exactly one accent (`drape`), and a ramp invites decorative colour.
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
        alarm: "var(--alarm)",
        caution: "var(--caution)",
      },
      fontFamily: {
        // Prose. A humanist sans, from the system — no webfont, so the app
        // renders identically offline and inside the Tauri CSP.
        sans: [
          "ui-sans-serif",
          "-apple-system",
          "Segoe UI",
          "Roboto",
          "Helvetica Neue",
          "sans-serif",
        ],
        // Data. A chart is a table; columns must align.
        mono: [
          "ui-monospace",
          "SFMono-Regular",
          "SF Mono",
          "Menlo",
          "Consolas",
          "Liberation Mono",
          "monospace",
        ],
      },
      borderRadius: {
        // The identity rules out rounded corners: structure comes from ruled
        // lines, the way a printed form does.
        none: "0",
      },
    },
  },
  plugins: [],
}
