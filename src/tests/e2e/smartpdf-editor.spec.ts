import { test, expect } from "@playwright/test";
import path from "node:path";

test.describe("SmartPDF — Advanced PDF Editor", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tools/pdf/editor");
  });

  test("loads dropzone screen with accurate Phase 1 title and privacy notice", async ({
    page,
  }) => {
    await expect(page.locator("h1")).toContainText("Advanced PDF Editor");
    await expect(
      page.getByText("Upload a PDF Document"),
    ).toBeVisible();
    await expect(
      page.getByText("All processing is 100% private in your browser"),
    ).toBeVisible();
  });

  test("uploads AcroForm PDF, renders viewer, detects form fields, and exports", async ({
    page,
  }) => {
    const fixturePath = path.join(
      process.cwd(),
      "test-assets",
      "smartpdf-form.pdf",
    );

    // Upload the AcroForm fixture
    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Select PDF File" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles(fixturePath);

    // Verify workspace mounts
    const workspace = page.locator(
      '[data-testid="smartpdf-editor-workspace"]',
    );
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Verify filename and page count in toolbar
    await expect(workspace).toContainText("smartpdf-form.pdf");
    await expect(workspace).toContainText("1 / 1");

    // Verify canvas is rendered
    const canvas = workspace.locator("canvas").first();
    await expect(canvas).toBeVisible();

    // Verify Form Fields inspector
    await expect(page.getByText("Form Fields")).toBeVisible();
    await expect(
      page.getByText("3 interactive fields detected"),
    ).toBeVisible();

    // Verify individual field inputs
    const fullNameInput = page.locator('input[placeholder="Enter text..."]');
    await expect(fullNameInput).toBeVisible();

    // Type into the text field
    await fullNameInput.fill("Harimau Malaya");

    // Verify modified badge appears
    await expect(page.getByText("Edited")).toBeVisible();

    // Test Export Editable PDF download
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("smartpdf-form-edited.pdf");

    // Test Export Flattened PDF via dropdown
    await page.getByLabel("More export options").click();
    const flattenedOption = page.getByRole("button", {
      name: "Export Flattened PDF",
    });
    await expect(flattenedOption).toBeVisible();

    const flattenedDownloadPromise = page.waitForEvent("download");
    await flattenedOption.click();
    const flattenedDownload = await flattenedDownloadPromise;
    expect(flattenedDownload.suggestedFilename()).toBe(
      "smartpdf-form-flattened.pdf",
    );
  });

  test("loads multi-page document and navigates between pages", async ({
    page,
  }) => {
    const fixturePath = path.join(
      process.cwd(),
      "test-assets",
      "multi-page.test.pdf",
    );

    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Select PDF File" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles(fixturePath);

    const workspace = page.locator(
      '[data-testid="smartpdf-editor-workspace"]',
    );
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Verify total page count
    await expect(workspace).toContainText("1 / 2");

    // Navigate to page 2 using Next Page button
    const nextBtn = page.getByLabel("Next Page");
    await nextBtn.click();
    await expect(workspace).toContainText("2 / 2");

    // Click page 1 thumbnail to jump back
    const page1Thumb = page.getByLabel("Page 1");
    await page1Thumb.click();
    await expect(workspace).toContainText("1 / 2");
  });

  test("opens document properties modal and displays correct metadata", async ({
    page,
  }) => {
    const fixturePath = path.join(
      process.cwd(),
      "test-assets",
      "smartpdf-form.pdf",
    );

    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Select PDF File" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles(fixturePath);

    await expect(
      page.locator('[data-testid="smartpdf-editor-workspace"]'),
    ).toBeVisible({ timeout: 10000 });

    // Open Document Properties
    await page.getByTitle("Document Properties").click();

    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText("Document Properties");
    await expect(dialog).toContainText("smartpdf-form.pdf");
    await expect(dialog).toContainText("1 page(s)");
    await expect(dialog).toContainText("3 field(s)");

    // Close modal
    await page.getByRole("button", { name: "Done" }).click();
    await expect(dialog).not.toBeVisible();
  });
});
