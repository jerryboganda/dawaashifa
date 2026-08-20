/**
 * Shifa Platform Design Tokens (Doc 16 §5)
 * Shared between web/apps and Tailwind configuration.
 */

export const colors = {
  brand: {
    50: "#f0fdfa",
    100: "#ccfbf1",
    200: "#99f6e4",
    300: "#5eead4",
    400: "#2dd4bf",
    500: "#14b8a6", // Primary brand teal
    600: "#0d9488",
    700: "#0f766e",
    800: "#115e59",
    900: "#134e4a",
  },
  surface: {
    base: "#ffffff",
    raised: "#f8fafc",
    sunken: "#f1f5f9",
    overlay: "rgba(15, 23, 42, 0.75)",
    darkBase: "#0f172a",
    darkRaised: "#1e293b",
  },
  text: {
    primary: "#0f172a",
    secondary: "#475569",
    muted: "#94a3b8",
    inverse: "#ffffff",
  },
  status: {
    pending: "#eab308", // Amber
    review: "#3b82f6",  // Blue
    approved: "#10b981", // Emerald green
    rejected: "#ef4444", // Red
    dispatched: "#8b5cf6", // Purple
    delivered: "#059669", // Dark emerald
    failed: "#dc2626", // Dark red
  },
  severity: {
    critical: "#dc2626", // Crimson
    high: "#ea580c",     // Orange
    medium: "#f59e0b",   // Amber
    low: "#3b82f6",      // Blue
  },
} as const;

export const spacing = {
  1: "4px",
  2: "8px",
  3: "12px",
  4: "16px",
  5: "20px",
  6: "24px",
  8: "32px",
  10: "40px",
  12: "48px",
  16: "64px",
} as const;

export const radius = {
  sm: "4px",
  md: "6px",
  lg: "10px",
  full: "9999px",
} as const;

export const fonts = {
  latin: "Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
  urdu: "'Noto Nastaliq Urdu', 'Jameel Noori Nastaleeq', Tahoma, sans-serif",
} as const;
