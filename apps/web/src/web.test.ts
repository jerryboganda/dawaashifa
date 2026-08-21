import { describe, it, expect } from "vitest";
import { PRODUCTS } from "./main";
import { formatPkr } from "@shifa/shared";

describe("Web Marketing & Public Portal Suite", () => {
  it("authenticates that all catalog items have valid MRP and manufacturers", () => {
    expect(PRODUCTS.length).toBeGreaterThan(0);
    for (const prod of PRODUCTS) {
      expect(prod.name).toBeTruthy();
      expect(prod.generic).toBeTruthy();
      expect(prod.mrp).toMatch(/^\d+(\.\d{2})?$/);
      expect(prod.manufacturer).toBeTruthy();
    }
  });

  it("ensures formatting of medicine prices strictly satisfies PKR string invariant", () => {
    const formatted = formatPkr("450.00");
    expect(formatted).toBe("Rs 450.00");

    const highValue = formatPkr("3200.00");
    expect(highValue).toBe("Rs 3,200.00");
  });

  it("validates prescription gating metadata for antibiotics and chronic medicines", () => {
    const augmentin = PRODUCTS.find((p) => p.name.includes("Augmentin"));
    expect(augmentin?.isRx).toBe(true);
    expect(augmentin?.category).toBe("ANTIBIOTIC");

    const panadol = PRODUCTS.find((p) => p.name.includes("Panadol"));
    expect(panadol?.isRx).toBe(false);
    expect(panadol?.category).toBe("OTC");
  });
});
