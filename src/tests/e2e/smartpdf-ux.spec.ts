import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const fixture = fs.readFileSync(path.join(process.cwd(),
  "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-markup-freetext.pdf"));

async function openFixture(page: import("@playwright/test").Page) {
  await page.goto("/smartpdf");
  const input = page.getByTestId("pdf-dropzone-input");
  await input.setInputFiles({ name: "ux.pdf", mimeType: "application/pdf", buffer: fixture });
  await expect(page.locator('canvas[data-rendered="true"]')).toBeVisible();
  const annotation = page.locator('[data-testid^="canvas-annotation-annot-obj-"][title*="FreeText"]').first();
  await expect(annotation).toBeVisible();
  await annotation.dispatchEvent("click");
  await expect(page.getByTestId("context-annotation-input")).toBeVisible();
}

test("FreeText drafts preview immediately, cancel rolls back, and export flushes one edit", async ({ page }) => {
  await openFixture(page);
  const input = page.getByTestId("context-annotation-input");
  const original = await input.inputValue();
  await input.fill("Rapid revised text");
  await expect(page.getByTestId("canvas-freetext-live-preview")).toHaveText("Rapid revised text");
  await page.getByTestId("context-freetext-bold").click();
  await expect(page.getByTestId("canvas-freetext-live-preview")).toHaveCSS("font-weight", "700");
  await page.getByTestId("context-freetext-italic").click();
  await expect(page.getByTestId("canvas-freetext-live-preview")).toHaveCSS("font-style", "italic");
  await page.keyboard.press("Escape");
  await expect(input).toBeHidden();

  await page.locator('[data-testid^="canvas-annotation-annot-obj-"][title*="FreeText"]').first().dispatchEvent("click");
  await expect(page.getByTestId("context-annotation-input")).toHaveValue(original);
  await page.getByTestId("context-annotation-input").fill("Final draft for export");
  await expect(page.getByTestId("canvas-freetext-live-preview")).toHaveText("Final draft for export");
  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export Editable" }).click();
  const download = await downloadPromise;
  const exported = fs.readFileSync((await download.path())!);
  expect(exported.length).toBeGreaterThan(100);
  await expect(page.getByTestId("context-annotation-input")).toBeHidden();
  await page.getByTestId("toolbar-undo-btn").click();
  await expect(page.locator('[data-testid^="canvas-annotation-annot-obj-"][title*="FreeText"]').first()).toBeVisible();
  await page.getByTestId("toolbar-redo-btn").click();

  await page.locator('input[aria-label="Open PDF document"]').setInputFiles({
    name: "ux-reopened.pdf", mimeType: "application/pdf", buffer: exported,
  });
  const confirm = page.getByTestId("confirm-dialog-confirm-btn");
  if (await confirm.isVisible().catch(() => false)) await confirm.click();
  await page.locator('[data-testid^="canvas-annotation-annot-obj-"][title*="FreeText"]').first().dispatchEvent("click");
  await expect(page.getByTestId("context-annotation-input")).toHaveValue("Final draft for export");
});

test("command palette searches semantically and supports keyboard navigation", async ({ page }) => {
  await openFixture(page);
  await page.keyboard.press("ControlOrMeta+k");
  await expect(page.getByTestId("smartpdf-command-palette")).toBeVisible();
  await page.getByTestId("smartpdf-command-query").fill("document info");
  await expect(page.getByTestId("smartpdf-command-properties")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("smartpdf-command-palette")).toBeHidden();
});

test("clicking the page commits the latest draft as one undo step", async ({ page }) => {
  await openFixture(page);
  const input = page.getByTestId("context-annotation-input");
  const original = await input.inputValue();
  await input.fill("Click away commit");
  await page.getByRole("main", { name: "PDF Document Page Viewport" }).click({ position: { x: 12, y: 12 } });
  await expect(input).toBeHidden();
  const annotation = page.locator('[data-testid^="canvas-annotation-annot-obj-"][title*="Click away commit"]').first();
  await expect(annotation).toBeVisible();
  await page.getByTestId("toolbar-undo-btn").click();
  await expect(page.locator(`[data-testid^="canvas-annotation-annot-obj-"][title*="${original}"]`).first()).toBeVisible();
  await page.getByTestId("toolbar-redo-btn").click();
  await expect(annotation).toBeVisible();
});
