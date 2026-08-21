import { defineConfig } from "vite";

export default defineConfig({
  base: "/rider/",
  test: {
    globals: true,
    environment: "node",
  },
});
