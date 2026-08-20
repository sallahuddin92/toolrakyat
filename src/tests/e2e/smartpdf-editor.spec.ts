import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import fontkit from "@pdf-lib/fontkit";
import { PDFDocument, rgb } from "pdf-lib";
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

async function createRichAppearanceFixture(): Promise<Buffer> {
  const document = await PDFDocument.create();
  document.registerFontkit(fontkit);
  const fontBytes = fs.readFileSync(
    path.join(process.cwd(), "node_modules/pdfjs-dist/standard_fonts/LiberationSans-Regular.ttf"),
  );
  const font = await document.embedFont(fontBytes, { subset: false });
  const page = document.addPage([612, 792]);
  const form = document.getForm();
  page.drawText("StarPDF v0.8 rich appearance fixture", {
    x: 50,
    y: 748,
    size: 18,
    font,
  });

  const embedded = form.createTextField("embedded");
  embedded.setText("Original embedded font");
  embedded.addToPage(page, {
    x: 50,
    y: 690,
    width: 510,
    height: 34,
    font,
    borderColor: rgb(0.1, 0.25, 0.55),
    borderWidth: 1,
  });
  embedded.defaultUpdateAppearances(font);

  const comb = form.createTextField("comb");
  comb.setMaxLength(6);
  comb.enableCombing();
  comb.setText("ABC123");
  comb.addToPage(page, {
    x: 50,
    y: 625,
    width: 300,
    height: 38,
    font,
    borderColor: rgb(0.55, 0.15, 0.15),
    borderWidth: 1,
  });
  comb.defaultUpdateAppearances(font);

  const multiline = form.createTextField("multiline");
  multiline.enableMultiline();
  multiline.setText("Original multiline text wraps inside the field.");
  multiline.addToPage(page, {
    x: 50,
    y: 510,
    width: 300,
    height: 90,
    font,
    borderColor: rgb(0.1, 0.45, 0.2),
    borderWidth: 1,
  });
  multiline.defaultUpdateAppearances(font);

  const list = form.createOptionList("list");
  list.enableMultiselect();
  list.setOptions(["Alpha", "Beta", "Gamma", "Delta"]);
  list.select(["Beta", "Delta"]);
  list.addToPage(page, {
    x: 380,
    y: 510,
    width: 180,
    height: 153,
    font,
    borderColor: rgb(0.25, 0.25, 0.25),
    borderWidth: 1,
  });
  list.defaultUpdateAppearances(font);

  return Buffer.from(await document.save({ useObjectStreams: false }));
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

  test("StarPDF worker regenerates and visibly renders representative v0.8 annotation appearances", async ({
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
        value: "StarPDF v0.8 visible text",
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
          rect: [40, 600, 230, 635],
          contents: "Before regeneration",
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
          rect: [250, 600, 335, 660],
          color: [0.8, 0, 0],
          fill_color: [1, 0.75, 0.75],
          border_width: 3,
        },
      });
      await request({
        type: "addAnnotation",
        handle,
        pageIndex: 0,
        input: {
          subtype: "Circle",
          rect: [350, 600, 440, 660],
          color: [0, 0.35, 0.8],
          fill_color: [0.75, 0.9, 1],
          border_width: 3,
        },
      });
      await request({
        type: "addAnnotation",
        handle,
        pageIndex: 0,
        input: {
          subtype: "Highlight",
          rect: [40, 650, 230, 672],
          quad_points: [40, 672, 230, 672, 40, 650, 230, 650],
          color: [1, 0.85, 0],
        },
      });
      await request({
        type: "addAnnotation",
        handle,
        pageIndex: 0,
        input: {
          subtype: "Line",
          rect: [250, 680, 440, 728],
          line_points: [260, 690, 430, 718],
          line_endings: ["OpenArrow", "ClosedArrow"],
          color: [0.1, 0.25, 0.75],
          fill_color: [0.9, 0.2, 0.2],
          border_width: 3,
          contents: "Detailed arrow line",
        },
      });

      const firstExport = await request({ type: "exportIncremental", handle });
      const firstOutput = firstExport.bytes as Uint8Array;
      const firstReopen = await request({
        type: "open",
        buffer: firstOutput.slice().buffer,
      });
      const firstAnnotations = await request({
        type: "getAnnotations",
        handle: firstReopen.handle as number,
        pageIndex: 0,
      });
      const freeText = (firstAnnotations.annotations as Array<{
        subtype: string;
        object_num: number;
        object_gen: number;
      }>).find((annotation) => annotation.subtype === "FreeText");
      if (!freeText) throw new Error("Expected exported FreeText annotation");
      await request({
        type: "updateAnnotation",
        handle: firstReopen.handle as number,
        objectNum: freeText.object_num,
        objectGen: freeText.object_gen,
        input: {
          contents: "FreeText AP regenerated",
          color: [0, 0.45, 0.2],
          border_width: 2,
        },
      });
      const secondExport = await request({
        type: "exportIncremental",
        handle: firstReopen.handle as number,
      });
      const output = secondExport.bytes as Uint8Array;
      const appearanceStatus = await request({
        type: "getAppearanceStatus",
        handle: firstReopen.handle as number,
      });
      const finalReopen = await request({
        type: "open",
        buffer: output.slice().buffer,
      });
      const annotations = await request({
        type: "getAnnotations",
        handle: finalReopen.handle as number,
        pageIndex: 0,
      });
      await request({ type: "close", handle });
      const staleHandle = await request({ type: "info", handle });
      await request({ type: "close", handle: firstReopen.handle as number });
      await request({ type: "close", handle: finalReopen.handle as number });
      worker.terminate();

      return {
        version: init.version,
        output: Array.from(output),
        prefixPreserved: output
          .slice(0, inputBytes.length)
          .every((value, index) => value === inputBytes[index]),
        annotationCount: (annotations.annotations as unknown[]).length,
        appearanceStatus: appearanceStatus.status,
        staleHandleCode: staleHandle.code,
      };
    }, Array.from(fixture));

    expect(workerResult.version).toBe("0.8.0");
    expect(workerResult.prefixPreserved).toBe(true);
    expect(workerResult.annotationCount).toBeGreaterThanOrEqual(5);
    expect(workerResult.appearanceStatus).toBe("AP_REGENERATED");
    expect(workerResult.staleHandleCode).toBe("INVALID_HANDLE");

    await page.reload();
    const mutatedCanvas = await uploadPdfBytes(
      page,
      "starpdf-v0.8-mutated.pdf",
      Buffer.from(workerResult.output),
    );
    const mutatedPng = await mutatedCanvas.screenshot();

    const originalImage = sharp(originalPng);
    const metadata = await originalImage.metadata();
    const imageWidth = metadata.width ?? 0;
    const imageHeight = metadata.height ?? 0;
    expect(imageWidth).toBeGreaterThan(450);
    expect(imageHeight).toBeGreaterThan(730);

    const region = {
      left: 35,
      top: imageHeight - 735,
      width: 410,
      height: 140,
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

  test("StarPDF visibly regenerates embedded-font, comb, multiline, and multi-select widgets", async ({
    page,
  }) => {
    const fixture = await createRichAppearanceFixture();
    const originalCanvas = await uploadPdfBytes(page, "rich-fields.pdf", fixture);
    const originalPng = await originalCanvas.screenshot();

    const workerResult = await page.evaluate(async (inputBytes) => {
      type WorkerMessage = Record<string, unknown> & {
        id: string;
        success?: boolean;
        type: string;
      };
      type Field = {
        field_type: string;
        name: string;
        object_num: number;
        object_gen: number;
        max_len?: number;
        is_comb: boolean;
        selected_indices: number[];
      };
      const worker = new Worker("/starpdf.worker.js", { type: "module" });
      let requestId = 0;
      const request = (message: Omit<WorkerMessage, "id">): Promise<WorkerMessage> => {
        const id = `rich-${++requestId}`;
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
      await request({ type: "init" });
      const open = await request({
        type: "open",
        buffer: new Uint8Array(inputBytes).buffer,
      });
      const handle = open.handle as number;
      const before = await request({ type: "getFormFields", handle });
      const fields = before.fields as Field[];
      const byName = (name: string) => {
        const field = fields.find((candidate) => candidate.name === name);
        if (!field) throw new Error(`Missing rich field: ${name}`);
        return field;
      };
      const embedded = byName("embedded");
      const comb = byName("comb");
      const multiline = byName("multiline");
      const list = byName("list");

      for (const [field, value] of [
        [embedded, "Embedded font regenerated"],
        [comb, "Z9Y8X7"],
        [multiline, "Explicit first line\nWrapped words continue across the bounded widget."],
      ] as const) {
        await request({
          type: "setTextField",
          handle,
          objectNum: field.object_num,
          objectGen: field.object_gen,
          value,
        });
      }
      await request({
        type: "setChoiceValues",
        handle,
        objectNum: list.object_num,
        objectGen: list.object_gen,
        values: ["Alpha", "Gamma"],
      });
      const exported = await request({ type: "exportIncremental", handle });
      const status = await request({ type: "getAppearanceStatus", handle });
      const output = exported.bytes as Uint8Array;
      const reopen = await request({ type: "open", buffer: output.slice().buffer });
      const after = await request({
        type: "getFormFields",
        handle: reopen.handle as number,
      });
      await request({ type: "close", handle });
      await request({ type: "close", handle: reopen.handle as number });
      worker.terminate();

      const finalFields = after.fields as Field[];
      return {
        output: Array.from(output),
        status: status.status,
        prefixPreserved: output
          .slice(0, inputBytes.length)
          .every((value, index) => value === inputBytes[index]),
        comb: {
          maxLen: comb.max_len,
          isComb: comb.is_comb,
        },
        selected: finalFields.find((field) => field.name === "list")?.selected_indices,
      };
    }, Array.from(fixture));

    expect(workerResult.status).toBe("AP_REGENERATED");
    expect(workerResult.prefixPreserved).toBe(true);
    expect(workerResult.comb).toEqual({ maxLen: 6, isComb: true });
    expect(workerResult.selected).toEqual([0, 2]);

    await page.reload();
    const mutatedCanvas = await uploadPdfBytes(
      page,
      "rich-fields-v0.8.pdf",
      Buffer.from(workerResult.output),
    );
    const mutatedPng = await mutatedCanvas.screenshot();
    const metadata = await sharp(originalPng).metadata();
    const region = {
      left: 45,
      top: 60,
      width: Math.min(520, (metadata.width ?? 0) - 45),
      height: 290,
    };
    expect(region.width).toBeGreaterThan(500);
    const originalRegion = await sharp(originalPng).extract(region).raw().toBuffer();
    const mutatedRegion = await sharp(mutatedPng).extract(region).raw().toBuffer();
    let changedChannels = 0;
    for (let index = 0; index < originalRegion.length; index += 1) {
      if (Math.abs(originalRegion[index] - mutatedRegion[index]) > 8) changedChannels += 1;
    }
    expect(changedChannels / originalRegion.length).toBeGreaterThan(0.01);
  });
});
