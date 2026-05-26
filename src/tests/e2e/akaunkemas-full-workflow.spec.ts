import { test, expect } from "@playwright/test";

// ---------------------------------------------------------------------------
// Hub page
// ---------------------------------------------------------------------------

test.describe("AkaunKemas Hub", () => {
  test("page loads with title and tool cards", async ({ page }) => {
    await page.goto("/tools/akaunkemas", { timeout: 60_000 });
    await expect(page.locator("h1")).toContainText("AkaunKemas");
    // Recommended Workflow card
    await expect(page.getByText("Recommended Workflow")).toBeVisible();
    // At least one tool card link
    await expect(page.locator("a[href^='/tools/akaunkemas/']").first()).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Receipt Organizer
// ---------------------------------------------------------------------------

test.describe("AkaunKemas Receipt Organizer", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tools/akaunkemas/receipt-organizer", { timeout: 60_000 });
  });

  test("page loads with title and empty state", async ({ page }) => {
    await expect(page.getByTestId("tool-page-main")).toBeVisible();
    await expect(page.locator("h1")).toContainText("Receipt Organizer");
    // No receipts empty state
    await expect(page.getByText("No receipts yet.")).toBeVisible();
  });

  test("add receipt form opens and can be cancelled", async ({ page }) => {
    // Click Add Receipt button
    await page.getByRole("button", { name: "Add Receipt" }).click();
    // Form fields should appear
    await expect(page.getByLabel("Merchant")).toBeVisible();
    await expect(page.getByRole("spinbutton", { name: "Amount (RM)", exact: true })).toBeVisible();
    // Cancel the form
    await page.getByRole("button", { name: "Cancel" }).click();
    // Form should close and empty state returns
    await expect(page.getByText("No receipts yet.")).toBeVisible();
  });

  test("can add a receipt and see it in the table", async ({ page }) => {
    await page.getByRole("button", { name: "Add Receipt" }).click();
    // Fill in merchant and amount
    await page.getByLabel("Merchant").fill("Kedai Ujian");
    await page.getByRole("spinbutton", { name: "Amount (RM)", exact: true }).fill("150.00");
    // Save
    await page.getByRole("button", { name: "Save" }).click();
    // Receipt should appear in table
    await expect(page.locator("table")).toBeVisible();
    await expect(page.locator("table")).toContainText("Kedai Ujian");
    // Summary should update
    await expect(page.getByText("Total Amount")).toBeVisible();
  });

  test("export buttons are visible after adding a receipt", async ({ page }) => {
    // Add a receipt first
    await page.getByRole("button", { name: "Add Receipt" }).click();
    await page.getByLabel("Merchant").fill("Test");
    await page.getByRole("spinbutton", { name: "Amount (RM)", exact: true }).fill("50.00");
    await page.getByRole("button", { name: "Save" }).click();
    // Export buttons should be visible
    await expect(page.getByRole("button", { name: "Receipt CSV", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Receipt JSON", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Receipt Summary PDF", exact: true })).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Simple Ledger
// ---------------------------------------------------------------------------

test.describe("AkaunKemas Simple Ledger", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tools/akaunkemas/simple-ledger", { timeout: 60_000 });
  });

  test("page loads with title and upload card", async ({ page }) => {
    await expect(page.getByTestId("tool-page-main")).toBeVisible();
    await expect(page.locator("h1")).toContainText("Simple Ledger");
    // Upload card with Choose CSV button
    await expect(page.getByText("Upload Bank CSV")).toBeVisible();
    await expect(page.getByRole("button", { name: "Choose CSV file" })).toBeVisible();
  });

  test("empty state shown when no data", async ({ page }) => {
    await expect(page.getByText("Upload a bank CSV to view ledger.")).toBeVisible();
  });

  test("can upload a sample CSV and see ledger entries", async ({ page }) => {
    const csvContent = `Date,Description,Debit,Credit,Balance\n2024-06-01,Sales transfer,,1200.00,15200.00\n2024-06-02,Office rent,500.00,,14700.00`;
    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose CSV file" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "ledger-test.csv",
      mimeType: "text/csv",
      buffer: Buffer.from(csvContent),
    });
    // Ledger table should appear with entries
    await expect(page.locator("table")).toBeVisible({ timeout: 15_000 });
    await expect(page.locator("table")).toContainText("Sales transfer");
    // Totals should show
    await expect(page.getByText("Total Debit")).toBeVisible();
    await expect(page.getByText("Total Credit")).toBeVisible();
    // Export buttons
    await expect(page.getByRole("button", { name: "Ledger CSV" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Ledger JSON" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Ledger PDF" })).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Accountant Pack
// ---------------------------------------------------------------------------

test.describe("AkaunKemas Accountant Pack", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tools/akaunkemas/accountant-pack", { timeout: 60_000 });
  });

  test("page loads with title and upload sections", async ({ page }) => {
    await expect(page.getByTestId("tool-page-main")).toBeVisible();
    await expect(page.locator("h1")).toContainText("Accountant Pack");
    // Upload sections
    await expect(page.getByText("1. Upload Bank CSV (Transactions)")).toBeVisible();
    await expect(page.getByText("2. Upload Receipt JSON (Optional)")).toBeVisible();
    await expect(page.getByText("3. Notes for Accountant (Optional)")).toBeVisible();
    // Empty state hint
    await expect(page.getByText("Upload a bank CSV to generate your accountant pack.")).toBeVisible();
  });

  test("can upload CSV and see Generate button", async ({ page }) => {
    const csvContent = `Date,Description,Debit,Credit,Balance\n2024-06-01,Sales,100.00,,10100.00\n2024-06-02,Rent,,200.00,10300.00`;
    const fileChooserPromise = page.waitForEvent("filechooser");
    // Click the first "Choose CSV file" button (in section 1)
    await page.getByRole("button", { name: "Choose CSV file" }).first().click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "pack-test.csv",
      mimeType: "text/csv",
      buffer: Buffer.from(csvContent),
    });
    // Summary should appear
    await expect(page.getByText("Total Income")).toBeVisible({ timeout: 15_000 });
    // Generate button should be visible
    await expect(page.getByRole("button", { name: /Generate Accountant Pack/ })).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Receipt Matcher
// ---------------------------------------------------------------------------

test.describe("AkaunKemas Receipt Matcher", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tools/akaunkemas/receipt-matcher", { timeout: 60_000 });
  });

  test("page loads with title and dual upload panels", async ({ page }) => {
    await expect(page.getByTestId("tool-page-main")).toBeVisible();
    await expect(page.locator("h1")).toContainText("Receipt Matcher");
    // Both upload panels
    await expect(page.getByText("Bank Transactions (CSV)")).toBeVisible();
    await expect(page.getByText("Receipts (CSV or JSON)")).toBeVisible();
    // Upload buttons
    await expect(page.getByRole("button", { name: /Choose Bank CSV/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /Choose Receipt File/ })).toBeVisible();
  });

  test("Run Matching button appears after uploading both files", async ({ page }) => {
    const bankCsv = `Date,Description,Debit,Credit,Balance\n2024-06-01,Payment,100.00,,9900.00`;
    const receiptCsv = `date,merchant,amount,payment_method,category,tax,service_charge,notes\n2024-06-01,Vendor,100.00,cash,purchases,0,0,`;

    // Upload bank CSV
    let fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: /Choose Bank CSV/ }).click();
    let fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "bank.csv",
      mimeType: "text/csv",
      buffer: Buffer.from(bankCsv),
    });

    // Upload receipt CSV
    fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: /Choose Receipt File/ }).click();
    fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "receipts.csv",
      mimeType: "text/csv",
      buffer: Buffer.from(receiptCsv),
    });

    // Date window and Run Matching button should appear
    await expect(page.getByRole("button", { name: /Run Matching/ })).toBeVisible({ timeout: 15_000 });
  });

  test("matching works end-to-end", async ({ page }) => {
    const bankCsv = `Date,Description,Debit,Credit,Balance\n2024-06-01,Payment to Supplier,100.00,,9900.00`;
    const receiptCsv = `date,merchant,amount,payment_method,category,tax,service_charge,notes\n2024-06-01,Supplier,100.00,cash,purchases,0,0,`;

    // Upload both files
    let fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: /Choose Bank CSV/ }).click();
    let fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({ name: "bank.csv", mimeType: "text/csv", buffer: Buffer.from(bankCsv) });

    fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: /Choose Receipt File/ }).click();
    fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({ name: "receipts.csv", mimeType: "text/csv", buffer: Buffer.from(receiptCsv) });

    // Run matching
    await page.getByRole("button", { name: /Run Matching/ }).click();

    // Results should appear — matched tab with count
    await expect(page.getByRole("tab", { name: /Matched \(\d+\)/ })).toBeVisible({ timeout: 10_000 });

    // Export buttons should be visible
    await expect(page.getByRole("button", { name: "Matched CSV", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Report JSON" })).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Cross-tool: Bank CSV Cleaner still works (regression)
// ---------------------------------------------------------------------------

test.describe("AkaunKemas Bank CSV Cleaner (regression)", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tools/akaunkemas/bank-csv-cleaner", { timeout: 60_000 });
  });

  test("page loads and accepts CSV upload", async ({ page }) => {
    await expect(page.getByTestId("tool-page-main")).toBeVisible();
    await expect(page.locator("h1")).toContainText("Bank CSV Cleaner");

    const csv = `Date,Description,Debit,Credit,Balance\n2024-06-01,Test,100.00,,9900.00`;
    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose CSV file" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({ name: "test.csv", mimeType: "text/csv", buffer: Buffer.from(csv) });

    await expect(page.locator("table")).toBeVisible({ timeout: 15_000 });
    await expect(page.locator("table")).toContainText("Test");
  });
});
