/** @type {import("prettier").Config} */
export default {
  semi: true,
  singleQuote: false,
  quoteProps: "consistent",
  trailingComma: "all",
  printWidth: 100,
  tabWidth: 2,
  useTabs: false,
  bracketSpacing: true,
  bracketSameLine: false,
  arrowParens: "always",
  endOfLine: "lf",
  plugins: ["prettier-plugin-tailwindcss"],
  tailwindStylesheet: "./src/index.css",
  overrides: [
    {
      files: ["*.json", "*.jsonc"],
      options: { trailingComma: "none" },
    },
  ],
};
