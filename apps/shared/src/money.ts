/**
 * Money utilities for Pakistan Rupee (PKR) formatted strings (Doc 16 §10, Invariant I-8).
 * Money is NEVER parsed as a JavaScript floating-point number.
 */

export function formatPkr(amountString: string | null | undefined): string {
  if (!amountString) return "Rs 0.00";
  const clean = amountString.trim();
  const isNegative = clean.startsWith("-");
  const abs = isNegative ? clean.slice(1) : clean;

  const parts = abs.split(".");
  const intPart = parts[0] || "0";
  let fracPart = parts[1] || "00";

  // Clamp fraction to 2 decimal places for display
  if (fracPart.length > 2) {
    fracPart = fracPart.slice(0, 2);
  } else while (fracPart.length < 2) {
    fracPart += "0";
  }

  // Format with thousand separators
  const formattedInt = intPart.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const prefix = isNegative ? "-Rs " : "Rs ";

  return `${prefix}${formattedInt}.${fracPart}`;
}

export function isValidMoneyString(value: string): boolean {
  return /^-?\d+(\.\d{1,4})?$/.test(value.trim());
}
