import { fontFamily } from 'tailwindcss/defaultTheme';

/** @type {import('tailwindcss').Config} */
export const content = ["./templates/*.html"];
export const theme = {
  extend: {
    fontFamily: {
      sans: ['Inter var', ...fontFamily.sans],
    },
  },
};
export const plugins = [require("daisyui")];
export const daisyui = {
  themes: ["light", "dark", "business"],
};
