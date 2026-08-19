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
        ink: {
          950: "#06080b",
          900: "#0b0e13",
          800: "#12161d",
          700: "#171c25",
          600: "#1d2330",
          500: "#262c37",
          400: "#4b5361",
          300: "#8b949e",
          100: "#d4dae2",
        },
        accent: {
          DEFAULT: "#1d9bf0",
          900: "#0a3a5c",
          700: "#0d5a96",
          500: "#1d9bf0",
          300: "#5cb2f5",
          100: "#c7e6fd",
        },
        gold: {
          DEFAULT: "#d4af37",
          900: "#5c4813",
          700: "#8a6d1f",
          500: "#d4af37",
          300: "#f0d580",
          100: "#fdf3d6",
        },
        semantic: {
          success: "#3fb950",
          warning: "#d29922",
          error: "#ff7b72",
        },
        primary: {
          DEFAULT: "#1d9bf0",
          50: "#c7e6fd",
          100: "#c7e6fd",
          400: "#5cb2f5",
          500: "#1d9bf0",
          600: "#0d5a96",
          700: "#0a3a5c",
        },
      },
      fontFamily: {
        display: ["Rubik Mono One", "monospace"],
        sans: ["Inter", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"],
        mono: ["JetBrains Mono", "SF Mono", "Monaco", "Consolas", "monospace"],
      },
      borderRadius: {
        cards: "0px",
      },
    },
  },
  plugins: [],
}
