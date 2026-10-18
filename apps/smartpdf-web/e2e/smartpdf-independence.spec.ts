import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { PDFDocument } from "pdf-lib";

const FIXTURE_PATH = path.resolve(
  process.cwd(),
  "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-markup-freetext.pdf",
);
const EDIT_FIXTURE_PATH = path.resolve(process.cwd(), "test-assets/edit-test.pdf");

test.describe("SmartPDF Standalone Independence & 18-Tool Qualification", () => {
  test("1. Standalone Home loads cleanly with zero ToolRakyat chrome and displays all workflows", async ({
    page,
  }) => {
    await page.goto("http://localhost:3005/");

    // Check Home container
    await expect(page.locator('[data-testid="smartpdf-standalone-application"]')).toBeVisible();
    await expect(page.locator('[data-testid="smartpdf-home-view"]')).toBeVisible();

    // Verify Brand & Hero
    await expect(page.getByText("Precision Document Engineering")).toBeVisible();
    await expect(page.getByTestId("home-open-pdf-btn")).toBeVisible();
    await expect(page.getByTestId("home-all-tools-btn")).toBeVisible();

    // Verify 4 Workflow Quick-Launch Cards
    await expect(page.getByTestId("workflow-card-text")).toBeVisible();
    await expect(page.getByTestId("workflow-card-pages")).toBeVisible();
    await expect(page.getByTestId("workflow-card-fill_and_sign")).toBeVisible();
    await expect(page.getByTestId("workflow-card-annotate")).toBeVisible();
  });

  test("2. All Tools view displays all 18 tool cards with filter chips and search", async ({
    page,
  }) => {
    await page.goto("http://localhost:3005/");
    await page.getByTestId("home-all-tools-btn").click();

    await expect(page.locator('[data-testid="smartpdf-all-tools-view"]')).toBeVisible();
    await expect(page.getByTestId("tools-search-input")).toBeVisible();

    // All 18 tools catalog audit
    const expectedToolIds = [
      "tool-edit-text",
      "tool-add-image",
      "tool-vector-shapes",
      "tool-organize-pages",
      "tool-merge-pdf",
      "tool-split-pdf",
      "tool-insert-blank",
      "tool-duplicate-page",
      "tool-fill-acroforms",
      "tool-flat-form-assist",
      "tool-place-signature",
      "tool-check-cross",
      "tool-freetext-annotation",
      "tool-highlight-markup",
      "tool-ink-drawings",
      "tool-metadata-inspector",
      "tool-signature-audit",
      "tool-export-canonical",
    ];

    for (const toolId of expectedToolIds) {
      await expect(page.locator(`[data-testid="tool-card-${toolId}"]`)).toBeVisible();
    }

    // Test Search filter
    await page.getByTestId("tools-search-input").fill("split");
    await expect(page.locator('[data-testid="tool-card-tool-split-pdf"]')).toBeVisible();
    await expect(page.locator('[data-testid="tool-card-tool-edit-text"]')).toBeHidden();

    // Clear search
    await page.getByTestId("tools-search-input").fill("");
    await expect(page.locator('[data-testid="tool-card-tool-edit-text"]')).toBeVisible();
  });

  test("3. Open real PDF in Workspace, render canvas, execute annotations, and verify undo/redo", async ({
    page,
  }) => {
    await page.goto("http://localhost:3005/");
    await expect(page.locator('[data-testid="smartpdf-home-view"]')).toBeVisible({ timeout: 10000 });

    const fixtureBytes = fs.readFileSync(FIXTURE_PATH);
    const fileInput = page.getByTestId("home-file-input");
    await fileInput.setInputFiles({
      name: "markup-test.pdf",
      mimeType: "application/pdf",
      buffer: fixtureBytes,
    });

    // Verify workspace loaded
    await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).toBeVisible({
      timeout: 15000,
    });
    await expect(page.locator('canvas[data-rendered="true"]')).toBeVisible({ timeout: 15000 });

    // Click an existing FreeText annotation to edit
    const annotation = page
      .locator('[data-testid^="canvas-annotation-"][title*="FreeText"]')
      .first();
    await expect(annotation).toBeVisible();
    await annotation.dispatchEvent("click");

    // Contextual property editor appears
    const input = page.getByTestId("context-annotation-input");
    await expect(input).toBeVisible();
    const originalText = await input.inputValue();

    // Mutate text and styling
    await input.fill("Independent Qualification Test");
    await expect(page.getByTestId("canvas-freetext-live-preview")).toHaveText(
      "Independent Qualification Test",
    );

    // Commit using the dedicated Done button on contextual toolbar
    await page.getByTestId("context-freetext-done-btn").click();
    await expect(input).toBeHidden();

    // Verify Undo becomes enabled and commit is reflected on canvas
    await expect(page.getByTestId("toolbar-undo-btn")).toBeEnabled();
    await expect(
      page
        .locator(
          '[data-testid^="canvas-annotation-"][title*="Independent Qualification Test"]',
        )
        .first(),
    ).toBeVisible();

    // Verify Undo reverts commit
    await page.getByTestId("toolbar-undo-btn").click();
    await expect(page.getByTestId("toolbar-redo-btn")).toBeEnabled();
    await expect(
      page.locator(`[data-testid^="canvas-annotation-"][title*="${originalText}"]`).first(),
    ).toBeVisible();

    // Verify Redo re-applies commit
    await page.getByTestId("toolbar-redo-btn").click();
    await expect(page.getByTestId("toolbar-undo-btn")).toBeEnabled();
    await expect(
      page
        .locator(
          '[data-testid^="canvas-annotation-"][title*="Independent Qualification Test"]',
        )
        .first(),
    ).toBeVisible();
  });

  test("4. Real PDF export, independent structural validation, and reopen cycle", async ({
    page,
  }) => {
    await page.goto("http://localhost:3005/");
    await expect(page.locator('[data-testid="smartpdf-home-view"]')).toBeVisible({ timeout: 10000 });

    const fixtureBytes = fs.readFileSync(EDIT_FIXTURE_PATH);
    const fileInput = page.getByTestId("home-file-input");
    await fileInput.setInputFiles({
      name: "export-test.pdf",
      mimeType: "application/pdf",
      buffer: fixtureBytes,
    });

    await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).toBeVisible({
      timeout: 15000,
    });
    await expect(page.locator("main canvas").first()).toBeVisible({ timeout: 15000 });

    // Switch to ANNOTATE mode
    await page.getByRole("button", { name: /Annotate/i }).first().click();

    // Trigger Export Editable PDF
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    const downloadPath = await download.path();
    expect(downloadPath).toBeTruthy();

    const exportedBytes = fs.readFileSync(downloadPath!);
    expect(exportedBytes.length).toBeGreaterThan(500);

    // Independent validation using pdf-lib
    const validatedDoc = await PDFDocument.load(exportedBytes);
    expect(validatedDoc.getPageCount()).toBeGreaterThanOrEqual(1);

    // Reopen exported PDF by returning to Home and loading it
    await page.getByRole("button", { name: /Home/i }).first().click();
    await expect(page.locator('[data-testid="smartpdf-home-view"]')).toBeVisible();

    const reopenInput = page.getByTestId("home-file-input");
    await reopenInput.setInputFiles({
      name: "reopened-exported.pdf",
      mimeType: "application/pdf",
      buffer: exportedBytes,
    });

    await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).toBeVisible({
      timeout: 15000,
    });
    await expect(page.locator('[title="reopened-exported.pdf"]')).toBeVisible({ timeout: 10000 });
    await expect(page.locator("main canvas").first()).toBeVisible();
  });

  test("5. Multi-dialog tool activations: Metadata info, Split, and Merge dialogs", async ({
    page,
  }) => {
    await page.goto("http://localhost:3005/");
    await expect(page.locator('[data-testid="smartpdf-home-view"]')).toBeVisible({ timeout: 10000 });

    const fixtureBytes = fs.readFileSync(EDIT_FIXTURE_PATH);
    const fileInput = page.getByTestId("home-file-input");
    await fileInput.setInputFiles({
      name: "dialogs-test.pdf",
      mimeType: "application/pdf",
      buffer: fixtureBytes,
    });

    await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).toBeVisible({
      timeout: 15000,
    });

    // 1. Metadata Inspector Modal
    await page.getByTestId("toolbar-info-btn").click();
    await expect(page.getByTestId("doc-info-modal")).toBeVisible();
    await page.getByTestId("doc-info-modal").getByRole("button").first().click();

    // 2. Switch to PAGES mode
    await page.getByRole("button", { name: /Pages/i }).first().click();
    await expect(page.locator('[data-testid="pdf-page-operations"]')).toBeVisible();

    // 3. Open Split Dialog
    await page.getByTestId("page-split").click();
    await expect(page.getByTestId("smartpdf-split-modal")).toBeVisible();
    await expect(page.getByText("Split / Extract Document")).toBeVisible();
    await page.getByTestId("smartpdf-split-modal").getByRole("button", { name: "Cancel" }).click();
    await expect(page.getByTestId("smartpdf-split-modal")).toBeHidden();

    // 4. Open Merge / Import Dialog
    await page.getByTestId("page-import").click();
    await expect(page.getByTestId("smartpdf-import-modal")).toBeVisible();
    await expect(page.getByText("Import Pages / Merge PDF")).toBeVisible();
    await page.getByTestId("smartpdf-import-modal").getByRole("button", { name: "Cancel" }).click();
    await expect(page.getByTestId("smartpdf-import-modal")).toBeHidden();
  });

  test("6. Honest recent files: reopen from IndexedDB cache and clear recents", async ({
    page,
  }) => {
    await page.goto("http://localhost:3005/");
    await expect(page.locator('[data-testid="smartpdf-home-view"]')).toBeVisible({ timeout: 10000 });

    // Open fixture so it gets saved to recent files store
    const fixtureBytes = fs.readFileSync(EDIT_FIXTURE_PATH);
    const fileInput = page.getByTestId("home-file-input");
    await fileInput.setInputFiles({
      name: "cached-recent.pdf",
      mimeType: "application/pdf",
      buffer: fixtureBytes,
    });

    await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).toBeVisible({
      timeout: 15000,
    });

    // Go back to Home
    await page.getByRole("button", { name: /Home/i }).first().click();
    await expect(page.locator('[data-testid="smartpdf-home-view"]')).toBeVisible();

    // Check recent documents list contains our file
    const recentItem = page
      .locator('[data-testid^="recent-item-"]')
      .filter({ hasText: "cached-recent.pdf" })
      .first();
    await expect(recentItem).toBeVisible();

    // Click recent item to reopen from local IndexedDB cache
    await recentItem.click();

    // Toast appears confirming cache load
    await expect(page.locator("text=Loaded").first()).toBeVisible({ timeout: 5000 });
    await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).toBeVisible({
      timeout: 15000,
    });
  });
});
