import { test, expect } from "@playwright/test";

const TEST_CSV = `Date,Description,Debit,Credit,Balance
2024-01-15,Payment to Supplier,1500.00,,8500.00
2024-01-16,Customer Payment,,3000.00,11500.00
2024-01-17,Utility Bill,200.00,,11300.00`;

test.describe("AkaunKemas Bank CSV Cleaner", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tools/akaunkemas/bank-csv-cleaner", { timeout: 60_000 });
  });

  test("page loads with title and description", async ({ page }) => {
    // Verify the tool-page shell is present
    await expect(page.getByTestId("tool-page-main")).toBeVisible();

    // Title is rendered in an h1 inside ToolPageShell
    await expect(page.locator("h1")).toContainText("Bank CSV Cleaner", {
      timeout: 30_000,
    });

    // Description paragraph
    await expect(
      page.locator("h1 + p").or(page.locator("p")).first(),
    ).toContainText("Upload a bank CSV", { timeout: 10_000 });
  });

  test("upload a sample CSV and see transactions appear", async ({ page }) => {
    // The file input is created dynamically when the "Choose CSV file" button is clicked.
    // Use Playwright's fileChooser API to intercept it.
    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose CSV file" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "test.csv",
      mimeType: "text/csv",
      buffer: Buffer.from(TEST_CSV),
    });

    // Wait for transactions table to appear
    await expect(page.locator("table")).toBeVisible({ timeout: 15_000 });

    // Verify transaction rows appear (3 transactions + 1 header row = 4 rows)
    const rows = page.locator("table tbody tr");
    await expect(rows).toHaveCount(3);

    // Check that one of the descriptions is visible
    await expect(page.locator("table")).toContainText("Payment to Supplier");
    await expect(page.locator("table")).toContainText("Customer Payment");
    await expect(page.locator("table")).toContainText("Utility Bill");
  });

  test("category dropdown can be changed", async ({ page }) => {
    // Upload CSV first
    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose CSV file" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "test.csv",
      mimeType: "text/csv",
      buffer: Buffer.from(TEST_CSV),
    });

    // Wait for table
    await expect(page.locator("table tbody tr")).toHaveCount(3, {
      timeout: 15_000,
    });

    // The first row has category "Tidak Pasti / Uncategorised" (default)
    // Click the first category select trigger (shadcn Select)
    const firstCategoryTrigger = page
      .locator("table tbody tr")
      .first()
      .locator('[role="combobox"]');

    await firstCategoryTrigger.click();

    // Select a different category from the dropdown
    await page
      .getByRole("option", { name: "Utiliti / Utilities" })
      .click();

    // Verify the select now shows the new category
    await expect(firstCategoryTrigger).toContainText("Utiliti / Utilities", {
      timeout: 5_000,
    });
  });

  test("summary card shows income, expense, and net values", async ({ page }) => {
    // Upload CSV first
    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose CSV file" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "test.csv",
      mimeType: "text/csv",
      buffer: Buffer.from(TEST_CSV),
    });

    // Wait for table
    await expect(page.locator("table tbody tr")).toHaveCount(3, {
      timeout: 15_000,
    });

    // Verify summary section is visible
    await expect(page.getByText("Total Income")).toBeVisible();
    await expect(page.getByText("Total Expenses")).toBeVisible();
    await expect(page.getByText("Net Cashflow")).toBeVisible();

    // Initially all transactions are "uncategorised" (neutral category).
    // Neutral transactions don't count as income or expense, so totals show 0.
    await expect(page.locator("div.text-lg.text-green-700").first()).toContainText("0.00");
    await expect(page.locator("div.text-lg.text-red-600").first()).toContainText("0.00");

    // Category breakdown should show "Tidak Pasti / Uncategorised" with all 3 transactions
    await expect(
      page.locator(".text-slate-700").filter({ hasText: "Tidak Pasti / Uncategorised" }).first(),
    ).toBeVisible();

    // Change the Customer Payment (row 2, credit 3000) to "Jualan / Sales"
    const customerPaymentRow = page.locator("table tbody tr").nth(1);
    await customerPaymentRow.locator('[role="combobox"]').click();
    await page.getByRole("option", { name: "Jualan / Sales" }).click();

    // Now total income should update to RM 3,000.00
    await expect(page.locator("div.text-lg.text-green-700").first()).toContainText("3,000.00");
  });

  test("export buttons are present", async ({ page }) => {
    // Upload CSV first
    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose CSV file" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles({
      name: "test.csv",
      mimeType: "text/csv",
      buffer: Buffer.from(TEST_CSV),
    });

    // Wait for table
    await expect(page.locator("table tbody tr")).toHaveCount(3, {
      timeout: 15_000,
    });

    // Verify all three export buttons exist
    await expect(
      page.getByRole("button", { name: "Cleaned CSV" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Summary JSON" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Monthly Summary PDF" }),
    ).toBeVisible();
  });
});
