/** @type {import('tailwindcss').Config} */
export default {
  darkMode: "class",
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Brand A (Cortex) per DESIGN_PACK.md Visual Tokens — the
        // default until Gate 0's brand pick is reconfirmed.
        primary: {
          DEFAULT: "#667eea",
          50: "#eef1fd",
          100: "#dde3fb",
          400: "#8b9cf0",
          500: "#667eea",
          600: "#4c5fd6",
          700: "#3c4ab0",
        },
      },
      fontFamily: {
        sans: ["Inter", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"],
        mono: ["SF Mono", "Monaco", "Consolas", "monospace"],
      },
    },
  },
  plugins: [],
}
