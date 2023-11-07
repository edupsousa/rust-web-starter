/** @type {import('tailwindcss').Config} */
export default {
  content: [
    './templates/**/*.html'
  ],
  theme: {
    extend: {},
  },
  plugins: [
    require("@tailwindcss/typography"),
    require('daisyui')
  ],
}

