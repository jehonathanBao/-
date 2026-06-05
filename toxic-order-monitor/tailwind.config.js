/** @type {import("tailwindcss").Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      boxShadow: {
        glow: "0 0 32px rgba(14, 165, 233, 0.12)",
      },
    },
  },
  plugins: [],
};
