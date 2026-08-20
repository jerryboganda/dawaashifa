import type { Config } from "tailwindcss";
import { colors, spacing, radius } from "@shifa/shared";

export default {
  content: ["./src/**/*.{html,js,svelte,ts}"],
  theme: {
    extend: {
      colors: {
        brand: colors.brand,
        surface: colors.surface,
        appText: colors.text,
        status: colors.status,
        severity: colors.severity,
      },
      spacing: spacing,
      borderRadius: radius,
    },
  },
  plugins: [],
} satisfies Config;
