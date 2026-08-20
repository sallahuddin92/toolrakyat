import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import sharp from "sharp";

async function uploadPdfBytes(
  page: import("@playwright/test").Page,
  name: string,
  bytes: Buffer,
) {
  await page.locator('input[type="file"]').setInputFiles({
    name,
    mimeType: "application/pdf",
    buffer: bytes,
  });
  const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
  await expect(workspace).toBeVisible({ timeout: 10000 });
  const canvas = workspace.locator("main canvas");
  await expect
    .poll(() => canvas.evaluate((node) => (node as HTMLCanvasElement).width))
    .toBeGreaterThan(0);
  return canvas;
}

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

  test("StarPDF worker mutates, incrementally exports, reopens, and visibly renders v0.7 appearances", async ({
    page,
  }) => {
    const fixture = fs.readFileSync(
      path.join(process.cwd(), "test-assets", "smartpdf-form.pdf"),
    );

    const originalCanvas = await uploadPdfBytes(
      page,
      "starpdf-original.pdf",
      fixture,
    );
    const originalPng = await originalCanvas.screenshot();

    const workerResult = await page.evaluate(async (inputBytes) => {
      type WorkerMessage = Record<string, unknown> & {
        id: string;
        success?: boolean;
        type: string;
      };

      const worker = new Worker("/starpdf.worker.js", { type: "module" });
      let requestId = 0;
      const request = (message: Omit<WorkerMessage, "id">): Promise<WorkerMessage> => {
        const id = `e2e-${++requestId}`;
        return new Promise((resolve, reject) => {
          const onMessage = (event: MessageEvent<WorkerMessage>) => {
            if (event.data.id !== id) return;
            worker.removeEventListener("message", onMessage);
            resolve(event.data);
          };
          worker.addEventListener("message", onMessage);
          worker.addEventListener("error", reject, { once: true });
          worker.postMessage({ ...message, id });
        });
      };

      const init = await request({ type: "init" });
      const open = await request({
        type: "open",
        buffer: new Uint8Array(inputBytes).buffer,
      });
      const handle = open.handle as number;
      const fieldResponse = await request({ type: "getFormFields", handle });
      const fields = fieldResponse.fields as Array<{
        field_type: string;
        object_num: number;
        object_gen: number;
        widgets: Array<{
          object_num: number;
          object_gen: number;
          normal_appearance_states: string[];
        }>;
      }>;

      const textField = fields.find((field) => field.field_type === "text");
      if (!textField) throw new Error("Expected a text field in visual fixture");
      await request({
        type: "setTextField",
        handle,
        objectNum: textField.object_num,
        objectGen: textField.object_gen,
        value: "StarPDF v0.7 visible text",
      });

      const checkbox = fields.find((field) => field.field_type === "checkbox");
      if (checkbox) {
        await request({
          type: "setCheckbox",
          handle,
          objectNum: checkbox.object_num,
          objectGen: checkbox.object_gen,
          checked: true,
        });
      }

      const radio = fields.find((field) => field.field_type === "radio");
      const radioWidget = radio?.widgets[0];
      const radioOnState = radioWidget?.normal_appearance_states.find(
        (state) => state !== "Off",
      );
      if (radio && radioWidget && radioOnState) {
        await request({
          type: "setRadio",
          handle,
          parentNum: radio.object_num,
          parentGen: radio.object_gen,
          widgetNum: radioWidget.object_num,
          widgetGen: radioWidget.object_gen,
          onState: radioOnState,
        });
      }

      await request({
        type: "addAnnotation",
        handle,
        pageIndex: 0,
        input: {
          subtype: "FreeText",
          rect: [40, 620, 230, 655],
          contents: "StarPDF annotation",
          font_size: 12,
          color: [0, 0, 0],
        },
      });
      await request({
        type: "addAnnotation",
        handle,
        pageIndex: 0,
        input: {
          subtype: "Square",
          rect: [40, 670, 230, 735],
          color: [0.8, 0, 0],
          fill_color: [1, 0.75, 0.75],
          border_width: 3,
        },
      });

      const exported = await request({ type: "exportIncremental", handle });
      const output = exported.bytes as Uint8Array;
      const reopen = await request({
        type: "open",
        buffer: output.slice().buffer,
      });
      const annotations = await request({
        type: "getAnnotations",
        handle: reopen.handle as number,
        pageIndex: 0,
      });
      await request({ type: "close", handle });
      const staleHandle = await request({ type: "info", handle });
      await request({ type: "close", handle: reopen.handle as number });
      worker.terminate();

      return {
        version: init.version,
        output: Array.from(output),
        prefixPreserved: output
          .slice(0, inputBytes.length)
          .every((value, index) => value === inputBytes[index]),
        annotationCount: (annotations.annotations as unknown[]).length,
        staleHandleCode: staleHandle.code,
      };
    }, Array.from(fixture));

    expect(workerResult.version).toBe("0.7.0");
    expect(workerResult.prefixPreserved).toBe(true);
    expect(workerResult.annotationCount).toBeGreaterThanOrEqual(2);
    expect(workerResult.staleHandleCode).toBe("INVALID_HANDLE");

    await page.reload();
    const mutatedCanvas = await uploadPdfBytes(
      page,
      "starpdf-v0.7-mutated.pdf",
      Buffer.from(workerResult.output),
    );
    const mutatedPng = await mutatedCanvas.screenshot();

    const originalImage = sharp(originalPng);
    const metadata = await originalImage.metadata();
    const imageWidth = metadata.width ?? 0;
    const imageHeight = metadata.height ?? 0;
    expect(imageWidth).toBeGreaterThan(230);
    expect(imageHeight).toBeGreaterThan(735);

    const region = {
      left: 35,
      top: imageHeight - 740,
      width: 205,
      height: 125,
    };
    const originalRegion = await sharp(originalPng).extract(region).raw().toBuffer();
    const mutatedRegion = await sharp(mutatedPng).extract(region).raw().toBuffer();
    let changedChannels = 0;
    for (let index = 0; index < originalRegion.length; index += 1) {
      if (Math.abs(originalRegion[index] - mutatedRegion[index]) > 8) {
        changedChannels += 1;
      }
    }
    expect(changedChannels / originalRegion.length).toBeGreaterThan(0.02);
  });
});
