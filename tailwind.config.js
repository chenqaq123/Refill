/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro Display",
          "SF Pro Text",
          "Inter",
          "Segoe UI",
          "sans-serif",
        ],
      },
      colors: {
        canvas: "#f7f8f8",
        panel: "#ffffff",
        muted: "#f1f2f2",
        line: "#e1e4e6",
        ink: "#202123",
        sub: "#71767d",
        blue: "#2477e8",
        green: "#22a861",
        teal: "#0a9a93",
        amber: "#cf7a24",
        red: "#c94a58",
      },
      boxShadow: {
        soft: "0 16px 48px rgba(28, 35, 45, 0.065)",
        card: "0 8px 24px rgba(28, 35, 45, 0.045)",
      },
    },
  },
  plugins: [],
};
