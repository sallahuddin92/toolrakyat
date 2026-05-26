import type { Receipt, CategorySlug, PaymentMethod } from "./types";

// ---------------------------------------------------------------------------
// Module-level ID counter
// ---------------------------------------------------------------------------

let receiptIdCounter = 0;

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Create a new Receipt with sensible defaults and an auto-incrementing ID.
 * Pass overrides to customise any field.
 */
export function createReceipt(overrides?: Partial<Receipt>): Receipt {
  receiptIdCounter += 1;
  const today = new Date().toISOString().slice(0, 10);
  return {
    id: receiptIdCounter,
    date: today,
    merchant: "",
    amount: 0,
    paymentMethod: "cash" as PaymentMethod,
    category: "uncategorised" as CategorySlug,
    taxAmount: 0,
    serviceCharge: 0,
    notes: "",
    imageRef: null,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// CRUD helpers (immutable array operations)
// ---------------------------------------------------------------------------

/** Append a receipt to the list (returns a new array). */
export function addReceipt(receipts: Receipt[], receipt: Receipt): Receipt[] {
  return [...receipts, receipt];
}

/** Replace the receipt with the given id, applying the supplied updates. */
export function updateReceipt(
  receipts: Receipt[],
  id: number,
  updates: Partial<Receipt>,
): Receipt[] {
  return receipts.map((r) => (r.id === id ? { ...r, ...updates } : r));
}

/** Remove the receipt with the given id (returns a new array). */
export function deleteReceipt(receipts: Receipt[], id: number): Receipt[] {
  return receipts.filter((r) => r.id !== id);
}

// ---------------------------------------------------------------------------
// Test helper
// ---------------------------------------------------------------------------

/** Reset the internal ID counter to zero (primarily for tests). */
export function resetReceiptIdCounter(): void {
  receiptIdCounter = 0;
}
