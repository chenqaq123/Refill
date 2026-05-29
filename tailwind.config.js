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
        canvas: "#f6f7f7",
        panel: "#ffffff",
        muted: "#f0f2f4",
        line: "#dfe3e7",
        ink: "#202123",
        sub: "#70757d",
        blue: "#2378ee",
        green: "#21a65b",
        teal: "#099d94",
        amber: "#d9822b",
        red: "#c84855",
      },
      boxShadow: {
        soft: "0 18px 60px rgba(28, 35, 45, 0.08)",
        card: "0 10px 30px rgba(28, 35, 45, 0.06)",
      },
    },
  },
  plugins: [],
};
