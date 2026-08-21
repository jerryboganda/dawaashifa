import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: process.env.VITEST ? [] : [svelte()],
  base: "/ops/",
  test: {
    globals: true,
    environment: "node",
  },
});
