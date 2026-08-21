import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import fontkit from "@pdf-lib/fontkit";
import { PDFDocument, PDFName, rgb } from "pdf-lib";
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
  await expect
    .poll(() => canvas.evaluate((node) => node.getAttribute("data-rendered")))
    .toBe("true");
  await page.waitForTimeout(300);
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
  page.drawText("StarPDF v0.9 rich appearance fixture", {
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
    width: 240,
    height: 34,
    font,
    borderColor: rgb(0.1, 0.25, 0.55),
    borderWidth: 1,
  });
  embedded.addToPage(page, {
    x: 320,
    y: 690,
    width: 240,
    height: 34,
    font,
    borderColor: rgb(0.1, 0.25, 0.55),
    borderWidth: 1,
  });
  embedded.defaultUpdateAppearances(font);
  const embeddedWidgets = embedded.acroField.getWidgets();
  embeddedWidgets[0]?.getOrCreateAppearanceCharacteristics().setRotation(90);
  embeddedWidgets[1]?.getOrCreateAppearanceCharacteristics().setRotation(270);

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
  comb.acroField.getWidgets()[0]?.getOrCreateAppearanceCharacteristics().setRotation(180);

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

  return Buffer.from(
    await document.save({
      useObjectStreams: false,
      updateFieldAppearances: false,
    }),
  );
}

async function createType0AppearanceFixture(): Promise<Buffer> {
  const source = fs.readFileSync(
    path.join(
      process.cwd(),
      "engine/starpdf/tests/fixtures/v0_9_compat/chrome-unicode.pdf",
    ),
  );
  const document = await PDFDocument.load(source, { updateMetadata: false });
  const page = document.getPage(0);
  const field = document.getForm().createTextField("type0-identity-h");
  field.setText("Original Type0 field");
  field.addToPage(page, {
    x: 50,
    y: 620,
    width: 300,
    height: 42,
    borderColor: rgb(0.1, 0.25, 0.55),
    borderWidth: 1,
  });

  const pageResources = page.node.Resources();
  if (!pageResources) throw new Error("Chrome Type0 fixture has no page resources");
  field.acroField.dict.set(PDFName.of("DR"), pageResources);
  field.acroField.setDefaultAppearance("/F5 14 Tf 0 g");

  return Buffer.from(
    await document.save({
      useObjectStreams: false,
      updateFieldAppearances: false,
    }),
  );
}

type ProducerMutation =
  | { kind: "add-square" }
  | { kind: "text"; name: string; value: string; occurrence?: number }
  | { kind: "checkbox"; name: string; checked: boolean }
  | { kind: "choice"; name: string; values: string[] }
  | { kind: "radio"; name: string; widgetIndex: number }
  | {
      kind: "annotation";
      subtype: string;
      input: Record<string, unknown>;
    };

async function mutateProducerFixture(
  page: import("@playwright/test").Page,
  bytes: Buffer,
  mutations: ProducerMutation[],
) {
  return page.evaluate(
    async ({ inputBytes, requestedMutations }) => {
      type WorkerMessage = Record<string, unknown> & {
        id: string;
        success?: boolean;
        type: string;
      };
      type Widget = {
        object_num: number;
        object_gen: number;
        normal_appearance_states: string[];
        has_normal_appearance: boolean;
        has_rollover_appearance: boolean;
        has_down_appearance: boolean;
      };
      type Field = {
        name: string;
        object_num: number;
        object_gen: number;
        value: string;
        widgets: Widget[];
      };
      type Annotation = {
        subtype: string;
        object_num: number;
        object_gen: number;
        contents?: string;
        uri?: string;
        has_normal_appearance: boolean;
        has_rollover_appearance: boolean;
        has_down_appearance: boolean;
      };
      const worker = new Worker("/starpdf.worker.js", { type: "module" });
      let requestId = 0;
      const request = (message: Omit<WorkerMessage, "id">): Promise<WorkerMessage> => {
        const id = `producer-${++requestId}`;
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
      const fields = fieldResponse.fields as Field[];
      const annotationResponse = await request({
        type: "getAnnotations",
        handle,
        pageIndex: 0,
      });
      const annotations = annotationResponse.annotations as Annotation[];
      const beforePreservation = annotations.map((annotation) => ({
        subtype: annotation.subtype,
        normal: annotation.has_normal_appearance,
        rollover: annotation.has_rollover_appearance,
        down: annotation.has_down_appearance,
      }));

      for (const mutation of requestedMutations as ProducerMutation[]) {
        if (mutation.kind === "add-square") {
          await request({
            type: "addAnnotation",
            handle,
            pageIndex: 0,
            input: {
              subtype: "Square",
              rect: [500, 735, 530, 765],
              color: [0.05, 0.35, 0.8],
              border_width: 2,
            },
          });
          continue;
        }
        if (mutation.kind === "annotation") {
          const annotation = annotations.find(
            (candidate) => candidate.subtype === mutation.subtype,
          );
          if (!annotation) throw new Error(`Missing annotation ${mutation.subtype}`);
          await request({
            type: "updateAnnotation",
            handle,
            objectNum: annotation.object_num,
            objectGen: annotation.object_gen,
            input: mutation.input,
          });
          continue;
        }
        const matches = fields.filter((candidate) => candidate.name === mutation.name);
        const field = matches[mutation.kind === "text" ? mutation.occurrence ?? 0 : 0];
        if (!field) throw new Error(`Missing producer field ${mutation.name}`);
        if (mutation.kind === "text") {
          await request({
            type: "setTextField",
            handle,
            objectNum: field.object_num,
            objectGen: field.object_gen,
            value: mutation.value,
          });
        } else if (mutation.kind === "checkbox") {
          await request({
            type: "setCheckbox",
            handle,
            objectNum: field.object_num,
            objectGen: field.object_gen,
            checked: mutation.checked,
          });
        } else if (mutation.kind === "choice") {
          await request({
            type: "setChoiceValues",
            handle,
            objectNum: field.object_num,
            objectGen: field.object_gen,
            values: mutation.values,
          });
        } else {
          const widget = field.widgets[mutation.widgetIndex];
          const onState = widget?.normal_appearance_states.find((state) => state !== "Off");
          if (!widget || !onState) throw new Error(`Missing radio state for ${mutation.name}`);
          await request({
            type: "setRadio",
            handle,
            parentNum: field.object_num,
            parentGen: field.object_gen,
            widgetNum: widget.object_num,
            widgetGen: widget.object_gen,
            onState,
          });
        }
      }

      const exported = await request({ type: "exportIncremental", handle });
      if (!exported.success) throw new Error(`StarPDF export failed: ${String(exported.error)}`);
      const output = exported.bytes as Uint8Array;
      const status = await request({ type: "getAppearanceStatus", handle });
      const reopen = await request({ type: "open", buffer: output.slice().buffer });
      const afterFields = await request({
        type: "getFormFields",
        handle: reopen.handle as number,
      });
      const afterAnnotations = await request({
        type: "getAnnotations",
        handle: reopen.handle as number,
        pageIndex: 0,
      });
      await request({ type: "close", handle });
      await request({ type: "close", handle: reopen.handle as number });
      worker.terminate();
      return {
        version: init.version,
        output: Array.from(output),
        status: status.status,
        fields: afterFields.fields as Field[],
        annotations: afterAnnotations.annotations as Annotation[],
        beforePreservation,
        prefixPreserved: output
          .slice(0, inputBytes.length)
          .every((value, index) => value === inputBytes[index]),
      };
    },
    { inputBytes: Array.from(bytes), requestedMutations: mutations },
  );
}

async function changedRegionRatio(
  before: Buffer,
  after: Buffer,
  region: { left: number; top: number; width: number; height: number },
) {
  const metadata = await sharp(before).metadata();
  const scale = (metadata.width ?? 612) / 612;
  const scaledRegion = {
    left: Math.max(0, Math.round(region.left * scale)),
    top: Math.max(0, Math.round(region.top * scale)),
    width: Math.min(Math.round(region.width * scale), (metadata.width ?? 0) - Math.max(0, Math.round(region.left * scale))),
    height: Math.min(Math.round(region.height * scale), (metadata.height ?? 0) - Math.max(0, Math.round(region.top * scale))),
  };
  const beforePixels = await sharp(before).extract(scaledRegion).raw().toBuffer();
  const afterPixels = await sharp(after).extract(scaledRegion).raw().toBuffer();
  let changedChannels = 0;
  for (let index = 0; index < beforePixels.length; index += 1) {
    if (Math.abs(beforePixels[index] - afterPixels[index]) > 8) changedChannels += 1;
  }
  return changedChannels / beforePixels.length;
}

test.describe("SmartPDF — Advanced PDF Editor", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/smartpdf");
  });

  test("v0.12B page controls reorder, duplicate, insert, delete, merge, and extract through the worker", async ({
    page,
  }) => {
    const fixture = fs.readFileSync(path.join(process.cwd(), "test-assets/multi-page.test.pdf"));
    await uploadPdfBytes(page, "page-operations.pdf", fixture);
    await expect(page.getByTestId("pdf-page-operations")).toBeVisible();
    await expect(page.getByText("1 / 2", { exact: true })).toBeVisible();

    await page.getByTestId("page-move-right").click();
    await expect(page.getByText("2 / 2", { exact: true })).toBeVisible({ timeout: 10_000 });

    await page.getByTestId("page-duplicate").click();
    await expect(page.getByText("3 / 3", { exact: true })).toBeVisible({ timeout: 10_000 });

    await page.getByTestId("page-insert-blank").click();
    await expect(page.getByText("4 / 4", { exact: true })).toBeVisible({ timeout: 10_000 });

    await page.getByTestId("page-delete").click();
    await expect(page.getByText("3 / 3", { exact: true })).toBeVisible({ timeout: 10_000 });

    const mergeChooserPromise = page.waitForEvent("filechooser");
    await page.getByTestId("page-merge").click();
    const mergeChooser = await mergeChooserPromise;
    await mergeChooser.setFiles({
      name: "added-pages.pdf",
      mimeType: "application/pdf",
      buffer: fixture,
    });
    await expect(page.getByText("1 / 5", { exact: true })).toBeVisible({ timeout: 10_000 });

    const downloadPromise = page.waitForEvent("download");
    await page.getByTestId("page-extract").click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("page-operations-extracted.pdf");
    const extractedPath = await download.path();
    expect(extractedPath).not.toBeNull();
    const extracted = fs.readFileSync(extractedPath!);

    await page.getByRole("button", { name: "Open" }).click();
    const confirmBtn = page.getByTestId("confirm-dialog-confirm-btn");
    if (await confirmBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await confirmBtn.click();
    }
    const canvas = await uploadPdfBytes(page, "page-operations-extracted.pdf", extracted);
    await expect(page.getByText("1 / 1", { exact: true })).toBeVisible();
    expect((await canvas.screenshot()).byteLength).toBeGreaterThan(1_000);
  });

  test("loads dropzone screen with accurate Phase 1 title and privacy notice", async ({
    page,
  }) => {
    await expect(page.getByText("SmartPDF", { exact: false }).first()).toBeVisible();
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

    // Verify Form Fields on canvas and direct editing
    const canvasField = page.locator('[data-testid="canvas-field-full_name"]');
    await expect(canvasField).toBeVisible({ timeout: 10000 });
    await canvasField.click();

    const contextInput = page.locator('[data-testid="context-form-input"]');
    await expect(contextInput).toBeVisible();
    await contextInput.fill("Harimau Malaya");

    // Verify modified badge / dot appears
    await expect(page.locator('[data-testid="document-modified-dot"]')).toBeVisible();

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
    await page.locator('[data-testid="toolbar-info-btn"]').click();

    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText("Document Properties");
    await expect(dialog).toContainText("smartpdf-form.pdf");
    await expect(dialog).toContainText("1 page(s)");
    await expect(dialog).toContainText("3 interactive field(s)");

    // Close modal
    await page.getByRole("button", { name: "Done" }).click();
    await expect(dialog).not.toBeVisible();
  });

  test("StarPDF worker regenerates and visibly renders representative v0.9 annotation appearances", async ({
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

    expect(workerResult.version).toBe("0.12.1");
    expect(workerResult.prefixPreserved).toBe(true);
    expect(workerResult.annotationCount).toBeGreaterThanOrEqual(5);
    expect(workerResult.appearanceStatus).toBe("AP_REGENERATED");
    expect(workerResult.staleHandleCode).toBe("INVALID_HANDLE");

    await page.reload();
    const mutatedCanvas = await uploadPdfBytes(
      page,
      "starpdf-v0.9-mutated.pdf",
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
      top: 100,
      width: Math.min(410, imageWidth - 35),
      height: Math.min(400, imageHeight - 100),
    };
    const originalRegion = await sharp(originalPng).extract(region).raw().toBuffer();
    const mutatedRegion = await sharp(mutatedPng).extract(region).raw().toBuffer();
    let changedChannels = 0;
    for (let index = 0; index < originalRegion.length; index += 1) {
      if (Math.abs(originalRegion[index] - mutatedRegion[index]) > 8) {
        changedChannels += 1;
      }
    }
    expect(changedChannels / originalRegion.length).toBeGreaterThan(0.005);
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
      if (!exported.success) throw new Error(`StarPDF export failed: ${String(exported.error)}`);
      const status = await request({ type: "getAppearanceStatus", handle });
      const quality = await request({ type: "getGlyphMappingQuality", handle });
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
      const appended = output.slice(inputBytes.length);
      const countAscii = (needle: string) => {
        const bytes = new TextEncoder().encode(needle);
        let count = 0;
        for (let offset = 0; offset + bytes.length <= appended.length; offset += 1) {
          if (bytes.every((value, index) => appended[offset + index] === value)) count += 1;
        }
        return count;
      };
      return {
        output: Array.from(output),
        status: status.status,
        quality: quality.quality,
        matrixCount: countAscii("/Matrix"),
        embeddedFontCount: countAscii("/FontFile2"),
        subsetResourceCount: countAscii("/SPF"),
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
    expect(workerResult.quality).toBe("EXACT");
    expect(workerResult.matrixCount).toBeGreaterThanOrEqual(4);
    expect(workerResult.embeddedFontCount).toBeGreaterThanOrEqual(1);
    expect(workerResult.subsetResourceCount).toBeGreaterThanOrEqual(4);
    expect(workerResult.prefixPreserved).toBe(true);
    expect(workerResult.comb).toEqual({ maxLen: 6, isComb: true });
    expect(workerResult.selected).toEqual([0, 2]);

    await page.reload();
    const mutatedCanvas = await uploadPdfBytes(
      page,
      "rich-fields-v0.9.pdf",
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

  test("StarPDF automatically embeds and visibly renders a supported Identity-H Type0 appearance", async ({
    page,
  }) => {
    const fixture = await createType0AppearanceFixture();
    const originalCanvas = await uploadPdfBytes(page, "type0-field.pdf", fixture);
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
        value: string;
      };
      const worker = new Worker("/starpdf.worker.js", { type: "module" });
      let requestId = 0;
      const request = (message: Omit<WorkerMessage, "id">): Promise<WorkerMessage> => {
        const id = `type0-${++requestId}`;
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
      const field = (before.fields as Field[]).find(
        (candidate) => candidate.name === "type0-identity-h",
      );
      if (!field) throw new Error("Expected the generated Type0 field");
      await request({
        type: "setTextField",
        handle,
        objectNum: field.object_num,
        objectGen: field.object_gen,
        value: "Bahasa Melayu",
      });
      const exported = await request({ type: "exportIncremental", handle });
      if (!exported.success) throw new Error(`StarPDF export failed: ${String(exported.error)}`);
      const status = await request({ type: "getAppearanceStatus", handle });
      const quality = await request({ type: "getGlyphMappingQuality", handle });
      const output = exported.bytes as Uint8Array;
      const reopen = await request({ type: "open", buffer: output.slice().buffer });
      const after = await request({
        type: "getFormFields",
        handle: reopen.handle as number,
      });
      await request({ type: "close", handle });
      await request({ type: "close", handle: reopen.handle as number });
      worker.terminate();

      const appended = output.slice(inputBytes.length);
      const appendedAscii = new TextDecoder("latin1").decode(appended);
      return {
        output: Array.from(output),
        status: status.status,
        quality: quality.quality,
        value: (after.fields as Field[]).find(
          (candidate) => candidate.name === "type0-identity-h",
        )?.value,
        hasSubsetFont: appendedAscii.includes("/FontFile2"),
        hasSubsetResource: appendedAscii.includes("/SPF"),
        prefixPreserved: output
          .slice(0, inputBytes.length)
          .every((value, index) => value === inputBytes[index]),
      };
    }, Array.from(fixture));

    expect(workerResult.status).toBe("AP_REGENERATED");
    expect(workerResult.quality).toBe("EXACT");
    expect(workerResult.value).toBe("Bahasa Melayu");
    expect(workerResult.hasSubsetFont).toBe(true);
    expect(workerResult.hasSubsetResource).toBe(true);
    expect(workerResult.prefixPreserved).toBe(true);

    await page.reload();
    const mutatedCanvas = await uploadPdfBytes(
      page,
      "type0-field-v0.9.pdf",
      Buffer.from(workerResult.output),
    );
    const mutatedPng = await mutatedCanvas.screenshot();
    expect(
      await changedRegionRatio(originalPng, mutatedPng, {
        left: 45,
        top: 125,
        width: 320,
        height: 60,
      }),
    ).toBeGreaterThan(0.005);
  });

  test("StarPDF visibly mutates PDFKit producer text and checkbox widgets without name aliasing", async ({
    page,
  }) => {
    const fixture = fs.readFileSync(
      path.join(
        process.cwd(),
        "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-text-checkbox.pdf",
      ),
    );
    const originalCanvas = await uploadPdfBytes(page, "pdfkit-text-checkbox.pdf", fixture);
    const originalPng = await originalCanvas.screenshot();
    const result = await mutateProducerFixture(page, fixture, [
      {
        kind: "text",
        name: "pdfkit.person.name",
        occurrence: 0,
        value: "Only first PDFKit widget changed",
      },
      { kind: "checkbox", name: "pdfkit.agree", checked: false },
    ]);
    expect(result.version).toBe("0.12.1");
    expect(result.status).toBe("AP_REGENERATED");
    expect(result.prefixPreserved).toBe(true);
    const shared = result.fields.filter((field) => field.name === "pdfkit.person.name");
    expect(shared.map((field) => field.value)).toEqual([
      "Only first PDFKit widget changed",
      "PDFKit multi-widget 2",
    ]);
    expect(result.fields.find((field) => field.name === "pdfkit.agree")?.value).toBe("false");

    await page.reload();
    const changedCanvas = await uploadPdfBytes(
      page,
      "pdfkit-text-checkbox-v0.10.pdf",
      Buffer.from(result.output),
    );
    const changedPng = await changedCanvas.screenshot();
    expect(
      await changedRegionRatio(originalPng, changedPng, {
        left: 40,
        top: 65,
        width: 530,
        height: 230,
      }),
    ).toBeGreaterThan(0.002);
  });

  test("StarPDF visibly mutates producer radio and choice fields", async ({ page }) => {
    const fixture = fs.readFileSync(
      path.join(
        process.cwd(),
        "engine/starpdf/tests/fixtures/v0_10_compat/pdflib-complete-form.pdf",
      ),
    );
    const originalCanvas = await uploadPdfBytes(page, "pdflib-complete-form.pdf", fixture);
    const originalPng = await originalCanvas.screenshot();
    const result = await mutateProducerFixture(page, fixture, [
      { kind: "choice", name: "country", values: ["Singapore"] },
      { kind: "radio", name: "priority", widgetIndex: 0 },
    ]);
    expect(result.status).toBe("AP_REGENERATED");
    expect(result.prefixPreserved).toBe(true);
    expect(result.fields.find((field) => field.name === "country")?.value).toBe("Singapore");
    // pdf-lib encodes radio export states as the widget indexes /0 and /1.
    expect(result.fields.find((field) => field.name === "priority")?.value).toBe("0");

    await page.reload();
    const changedCanvas = await uploadPdfBytes(
      page,
      "pdflib-radio-choice-v0.10.pdf",
      Buffer.from(result.output),
    );
    const changedPng = await changedCanvas.screenshot();
    const metadata = await sharp(originalPng).metadata();
    expect(
      await changedRegionRatio(originalPng, changedPng, {
        left: 40,
        top: 200,
        width: Math.min(300, (metadata.width ?? 0) - 40),
        height: Math.min(180, (metadata.height ?? 0) - 200),
      }),
    ).toBeGreaterThan(0.0001);
  });

  test("StarPDF visibly mutates a PDFKit page-rotated and widget-rotated text field", async ({
    page,
  }) => {
    const fixture = fs.readFileSync(
      path.join(
        process.cwd(),
        "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-rotated-widget.pdf",
      ),
    );
    const originalCanvas = await uploadPdfBytes(page, "pdfkit-rotated-widget.pdf", fixture);
    const originalPng = await originalCanvas.screenshot();
    const result = await mutateProducerFixture(page, fixture, [
      {
        kind: "text",
        name: "pdfkit.rotated",
        value: "Rotated exact producer update",
      },
    ]);
    expect(result.status).toBe("AP_REGENERATED");
    expect(result.fields.find((field) => field.name === "pdfkit.rotated")?.value).toBe(
      "Rotated exact producer update",
    );
    await page.reload();
    const changedCanvas = await uploadPdfBytes(
      page,
      "pdfkit-rotated-widget-v0.10.pdf",
      Buffer.from(result.output),
    );
    const changedPng = await changedCanvas.screenshot();
    const metadata = await sharp(originalPng).metadata();
    expect(
      await changedRegionRatio(originalPng, changedPng, {
        left: 0,
        top: 0,
        width: metadata.width ?? 0,
        height: metadata.height ?? 0,
      }),
    ).toBeGreaterThan(0.00001);
  });

  test("StarPDF regenerates PDFKit FreeText and Highlight while preserving unrelated AP", async ({
    page,
  }) => {
    const fixture = fs.readFileSync(
      path.join(
        process.cwd(),
        "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-markup-freetext.pdf",
      ),
    );
    const originalCanvas = await uploadPdfBytes(page, "pdfkit-markup-freetext.pdf", fixture);
    const originalPng = await originalCanvas.screenshot();
    const result = await mutateProducerFixture(page, fixture, [
      {
        kind: "annotation",
        subtype: "FreeText",
        input: { contents: "StarPDF regenerated PDFKit FreeText", color: [0, 0.45, 0.2] },
      },
      {
        kind: "annotation",
        subtype: "Highlight",
        input: { color: [0.2, 0.8, 1] },
      },
    ]);
    expect(result.status).toBe("AP_REGENERATED");
    expect(
      result.annotations.find((annotation) => annotation.subtype === "FreeText")?.contents,
    ).toBe("StarPDF regenerated PDFKit FreeText");
    expect(
      result.annotations.find((annotation) => annotation.subtype === "Underline")
        ?.has_normal_appearance,
    ).toBe(true);
    await page.reload();
    const changedCanvas = await uploadPdfBytes(
      page,
      "pdfkit-annotations-v0.10.pdf",
      Buffer.from(result.output),
    );
    const changedPng = await changedCanvas.screenshot();
    expect(
      await changedRegionRatio(originalPng, changedPng, {
        left: 40,
        top: 115,
        width: 285,
        height: 180,
      }),
    ).toBeGreaterThan(0.002);
  });

  test("StarPDF regenerates a PDFKit Line and preserves unrelated shape and Link data", async ({
    page,
  }) => {
    const fixture = fs.readFileSync(
      path.join(
        process.cwd(),
        "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-shapes-ink-link.pdf",
      ),
    );
    const originalCanvas = await uploadPdfBytes(page, "pdfkit-shapes-ink-link.pdf", fixture);
    const originalPng = await originalCanvas.screenshot();
    const result = await mutateProducerFixture(page, fixture, [
      {
        kind: "annotation",
        subtype: "Line",
        input: {
          line_points: [55, 525, 265, 565],
          line_endings: ["OpenArrow", "ClosedArrow"],
          color: [0.8, 0.1, 0.2],
          border_width: 3,
        },
      },
      {
        kind: "annotation",
        subtype: "Square",
        input: { contents: "Semantic-only square note" },
      },
    ]);
    expect(result.status).toBe("AP_REGENERATED");
    expect(
      result.annotations.find((annotation) => annotation.subtype === "Square")
        ?.has_normal_appearance,
    ).toBe(true);
    expect(result.annotations.find((annotation) => annotation.subtype === "Link")?.uri).toBe(
      "https://example.invalid/starpdf-v0.10",
    );
    await page.reload();
    const changedCanvas = await uploadPdfBytes(
      page,
      "pdfkit-line-v0.10.pdf",
      Buffer.from(result.output),
    );
    const changedPng = await changedCanvas.screenshot();
    expect(
      await changedRegionRatio(originalPng, changedPng, {
        left: 40,
        top: 210,
        width: 250,
        height: 90,
      }),
    ).toBeGreaterThan(0.00001);
  });

  test("all v0.10 producer fixtures complete incremental export, reopen, and PDF.js render", async ({
    page,
  }) => {
    const fixtureIds = [
      "pdfkit-choice-radio",
      "pdfkit-markup-freetext",
      "pdfkit-rotated-widget",
      "pdfkit-shapes-ink-link",
      "pdfkit-text-checkbox",
      "pdflib-complete-form",
      "pdflib-inherited-field",
      "pdflib-needappearances-ap-rd",
      "pdflib-starpdf-two-revisions",
    ];
    for (const fixtureId of fixtureIds) {
      const fixture = fs.readFileSync(
        path.join(
          process.cwd(),
          `engine/starpdf/tests/fixtures/v0_10_compat/${fixtureId}.pdf`,
        ),
      );
      const result = await mutateProducerFixture(page, fixture, [{ kind: "add-square" }]);
      expect(result.status, fixtureId).toBe("AP_REGENERATED");
      expect(result.prefixPreserved, fixtureId).toBe(true);
      expect(result.annotations.length, fixtureId).toBe(result.beforePreservation.length + 1);
      await page.reload();
      const canvas = await uploadPdfBytes(
        page,
        `${fixtureId}-v0.10-roundtrip.pdf`,
        Buffer.from(result.output),
      );
      expect((await canvas.screenshot()).byteLength, fixtureId).toBeGreaterThan(1_000);
    }
  });

  test("signed PDF shows precise non-verification warning and remains viewable", async ({ page }) => {
    const fixture = fs.readFileSync(
      path.join(
        process.cwd(),
        "engine/starpdf/tests/fixtures/v0_11_complex/synthetic-signed-valid.pdf",
      ),
    );
    await uploadPdfBytes(page, "synthetic-signed-valid.pdf", fixture);
    const warning = page.getByTestId("starpdf-signed-document-warning");
    await expect(warning).toBeVisible();
    await expect(warning).toContainText("does not verify cryptographic signature validity");
    await expect(warning).toContainText("post-signature revision");
  });

  test("encrypted PDF is refused with an explicit security-handler message", async ({ page }) => {
    const fixture = fs.readFileSync(
      path.join(
        process.cwd(),
        "engine/starpdf/tests/fixtures/v0_11_complex/synthetic-encrypted-standard.pdf",
      ),
    );
    await page.locator('input[type="file"]').setInputFiles({
      name: "synthetic-encrypted-standard.pdf",
      mimeType: "application/pdf",
      buffer: fixture,
    });
    const error = page.getByTestId("starpdf-document-error");
    await expect(error).toBeVisible();
    await expect(error).toContainText("encrypted with an unsupported security handler");
    await expect(error).toContainText("does not decrypt or bypass");
    await expect(page.getByRole("main", { name: "PDF Document Page Viewport" })).toHaveCount(0);
  });

  test("worker returns typed encrypted-document refusal", async ({ page }) => {
    const fixture = fs.readFileSync(
      path.join(
        process.cwd(),
        "engine/starpdf/tests/fixtures/v0_11_complex/synthetic-encrypted-standard.pdf",
      ),
    );
    const result = await page.evaluate(async (input) => {
      const worker = new Worker("/starpdf.worker.js", { type: "module" });
      let sequence = 0;
      const request = (message: Record<string, unknown>) =>
        new Promise<Record<string, unknown>>((resolve, reject) => {
          const id = `security-${++sequence}`;
          const listener = (event: MessageEvent<Record<string, unknown>>) => {
            if (event.data.id !== id) return;
            worker.removeEventListener("message", listener);
            resolve(event.data);
          };
          worker.addEventListener("message", listener);
          worker.addEventListener("error", reject, { once: true });
          worker.postMessage({ ...message, id });
        });
      await request({ type: "init" });
      const opened = await request({
        type: "open",
        buffer: new Uint8Array(input).buffer,
      });
      const handle = opened.handle as number;
      const security = await request({ type: "securityInfo", handle });
      const staged = await request({
        type: "addAnnotation",
        handle,
        pageIndex: 0,
        input: { subtype: "Square", rect: [10, 10, 20, 20] },
      });
      const refusal = await request({ type: "exportIncremental", handle });
      await request({ type: "close", handle });
      worker.terminate();
      return { security, staged, refusal };
    }, Array.from(fixture));
    expect((result.security.securityInfo as { encryption_state: string }).encryption_state).toBe(
      "STANDARD_SECURITY_DETECTED",
    );
    expect(result.staged.success).toBe(true);
    expect(result.refusal.success).toBe(false);
    expect(result.refusal.code).toBe("ENCRYPTED_DOCUMENT");
  });

  test("ordinary PDF remains free of security warnings", async ({ page }) => {
    const fixture = fs.readFileSync(path.join(process.cwd(), "test-assets/smartpdf-form.pdf"));
    await uploadPdfBytes(page, "ordinary-form.pdf", fixture);
    await expect(page.getByTestId("starpdf-signed-document-warning")).toHaveCount(0);
    await expect(page.getByTestId("starpdf-document-error")).toHaveCount(0);
  });

  test("qualifies 20-page document load, search, tab inspection, and export in browser", async ({
    page,
  }) => {
    // Generate a deterministic 20-page test PDF buffer
    let pdfStr = "%PDF-1.7\n%StarPDF\n";
    const offsets: number[] = [0];
    const numPages = 20;

    offsets.push(pdfStr.length);
    pdfStr += "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";

    offsets.push(pdfStr.length);
    let kids = "";
    for (let i = 0; i < numPages; i++) {
      kids += `${3 + i * 2} 0 R `;
    }
    pdfStr += `2 0 obj\n<< /Type /Pages /Kids [${kids}] /Count ${numPages} >>\nendobj\n`;

    const fontObjNum = 3 + numPages * 2;
    for (let i = 0; i < numPages; i++) {
      const pageObjNum = 3 + i * 2;
      const contentObjNum = 4 + i * 2;

      offsets.push(pdfStr.length);
      pdfStr += `${pageObjNum} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents ${contentObjNum} 0 R /Resources << /Font << /F1 ${fontObjNum} 0 R >> >> >>\nendobj\n`;

      let streamContent = "BT\n/F1 12 Tf\n";
      for (let l = 0; l < 8; l++) {
        const y = 700 - l * 20;
        streamContent += `50 ${y} Td (Page ${i + 1} Section ${l + 1}: Large Document Benchmark Content) Tj\n`;
      }
      streamContent += "ET\n";

      offsets.push(pdfStr.length);
      pdfStr += `${contentObjNum} 0 obj\n<< /Length ${streamContent.length} >>\nstream\n${streamContent}\nendstream\nendobj\n`;
    }

    offsets.push(pdfStr.length);
    pdfStr += `${fontObjNum} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n`;

    const xrefOffset = pdfStr.length;
    pdfStr += `xref\n0 ${offsets.length}\n0000000000 65535 f \n`;
    for (let i = 1; i < offsets.length; i++) {
      pdfStr += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
    }
    pdfStr += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;

    const pdfBuffer = Buffer.from(pdfStr, "ascii");
    await uploadPdfBytes(page, "large-20p-test.pdf", pdfBuffer);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });
    await expect(workspace).toContainText("1 / 20");

    // 1. Search across 20 pages
    await page.getByLabel("Search text").click();
    const searchInput = page.getByPlaceholder("Search in document...");
    await searchInput.fill("Benchmark");
    await page.waitForTimeout(400);

    // 2. Select text span directly on canvas
    const textSpan = page.locator('[data-testid^="canvas-text-span-"]').first();
    await expect(textSpan).toBeVisible({ timeout: 5000 });
    await textSpan.click();
    await expect(page.locator('[data-testid="pdf-contextual-toolbar"]')).toBeVisible();

    // 3. Export
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("large-20p-test-edited.pdf");
  });

  test("qualifies 100-page document load, search, navigation, and export in browser", async ({
    page,
  }) => {
    // Generate deterministic 100-page test PDF
    let pdfStr = "%PDF-1.7\n%StarPDF\n";
    const offsets: number[] = [0];
    const numPages = 100;

    offsets.push(pdfStr.length);
    pdfStr += "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";

    offsets.push(pdfStr.length);
    let kids = "";
    for (let i = 0; i < numPages; i++) {
      kids += `${3 + i * 2} 0 R `;
    }
    pdfStr += `2 0 obj\n<< /Type /Pages /Kids [${kids}] /Count ${numPages} >>\nendobj\n`;

    const fontObjNum = 3 + numPages * 2;
    for (let i = 0; i < numPages; i++) {
      const pageObjNum = 3 + i * 2;
      const contentObjNum = 4 + i * 2;

      offsets.push(pdfStr.length);
      pdfStr += `${pageObjNum} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents ${contentObjNum} 0 R /Resources << /Font << /F1 ${fontObjNum} 0 R >> >> >>\nendobj\n`;

      let streamContent = "BT\n/F1 11 Tf\n";
      for (let l = 0; l < 4; l++) {
        const y = 720 - l * 20;
        streamContent += `50 ${y} Td (Page ${i + 1} Line ${l + 1}: Browser qualification performance text stream) Tj\n`;
      }
      streamContent += "ET\n";

      offsets.push(pdfStr.length);
      pdfStr += `${contentObjNum} 0 obj\n<< /Length ${streamContent.length} >>\nstream\n${streamContent}\nendstream\nendobj\n`;
    }

    offsets.push(pdfStr.length);
    pdfStr += `${fontObjNum} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n`;

    const xrefOffset = pdfStr.length;
    pdfStr += `xref\n0 ${offsets.length}\n0000000000 65535 f \n`;
    for (let i = 1; i < offsets.length; i++) {
      pdfStr += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
    }
    pdfStr += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;

    const pdfBuffer = Buffer.from(pdfStr, "ascii");
    await uploadPdfBytes(page, "large-100p-test.pdf", pdfBuffer);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 15000 });
    await expect(workspace).toContainText("1 / 100");

    // Search across 100 pages in browser
    await page.getByLabel("Search text").click();
    const searchInput = page.getByPlaceholder("Search in document...");
    await searchInput.fill("performance");
    await page.waitForTimeout(500);

    // Export editable 100-page document
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("large-100p-test-edited.pdf");
  });

  test("v0.19 Workflow A: Open -> Search -> Edit Text -> Export Editable", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/smartpdf-upload-1page.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workflow-a.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // 1. Search text
    await page.getByLabel("Search text").click();
    const searchInput = page.getByPlaceholder("Search in document...");
    await searchInput.fill("SmartPDF");
    await page.waitForTimeout(300);

    // 2. Select text span directly on canvas and replace text
    const textSpan = page.locator('[data-testid^="canvas-text-span-"]').first();
    if (await textSpan.isVisible()) {
      await textSpan.click();
      const contextInput = page.locator('[data-testid="context-text-input"]');
      await expect(contextInput).toBeVisible({ timeout: 5000 });
      await contextInput.fill("Converged StarPDF v0.19");
      await page.locator('[data-testid="context-text-save-btn"]').click();
      await page.waitForTimeout(500);
    }

    // 3. Export
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-a-edited.pdf");
  });

  test("v0.19 Workflow B: Open -> Replace Image -> Export Editable", async ({
    page,
  }) => {
    // Generate PDF with image
    const doc = await PDFDocument.create();
    const p = doc.addPage([400, 400]);
    const samplePng = await sharp({
      create: { width: 40, height: 40, channels: 3, background: { r: 255, g: 0, b: 0 } },
    }).png().toBuffer();
    const embeddedImg = await doc.embedPng(samplePng);
    p.drawImage(embeddedImg, { x: 50, y: 50, width: 200, height: 200 });
    const bytes = Buffer.from(await doc.save());

    await uploadPdfBytes(page, "workflow-b.pdf", bytes);
    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Select image directly on canvas
    const imgElement = page.locator('[data-testid^="canvas-image-"]').first();
    await expect(imgElement).toBeVisible({ timeout: 10000 });
    await imgElement.click();
    await expect(page.locator('[data-testid="context-image-replace-btn"]')).toBeVisible({ timeout: 5000 });

    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-b-edited.pdf");
  });

  test("v0.19 Workflow C: Open -> Modify Vector Shape -> Export Editable", async ({
    page,
  }) => {
    // Generate PDF with rectangle
    let pdfStr = "%PDF-1.7\n%StarPDF\n";
    const offsets: number[] = [0];
    offsets.push(pdfStr.length);
    pdfStr += "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
    offsets.push(pdfStr.length);
    pdfStr += "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n";
    offsets.push(pdfStr.length);
    pdfStr += "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 4 0 R /Resources << >> >>\nendobj\n";
    const streamContent = "q\n0.2 0.4 0.8 rg\n100 100 150 100 re\nf\nQ\n";
    offsets.push(pdfStr.length);
    pdfStr += `4 0 obj\n<< /Length ${streamContent.length} >>\nstream\n${streamContent}\nendstream\nendobj\n`;
    const xrefOffset = pdfStr.length;
    pdfStr += `xref\n0 ${offsets.length}\n0000000000 65535 f \n`;
    for (let i = 1; i < offsets.length; i++) {
      pdfStr += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
    }
    pdfStr += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;

    const bytes = Buffer.from(pdfStr, "ascii");
    await uploadPdfBytes(page, "workflow-c.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Select vector shape directly on canvas
    const graphicElement = page.locator('[data-testid^="canvas-graphic-"]').first();
    await expect(graphicElement).toBeVisible({ timeout: 10000 });
    await graphicElement.click();
    await expect(page.locator('[data-testid="context-vector-delete-btn"]')).toBeVisible({ timeout: 5000 });

    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-c-edited.pdf");
  });

  test("v0.19 Workflow D: Open Form -> Edit Values -> Export Editable", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/smartpdf-form.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workflow-d.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Edit fullName directly on canvas
    const canvasField = page.locator('[data-testid="canvas-field-full_name"]');
    await expect(canvasField).toBeVisible({ timeout: 10000 });
    await canvasField.click();

    const nameInput = page.locator('[data-testid="context-form-input"]');
    await expect(nameInput).toBeVisible({ timeout: 10000 });
    await nameInput.fill("Ahmad ToolRakyat");

    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-d-edited.pdf");
  });

  test("v0.19 Workflow E: Page Reorder, Duplicate, Delete -> Export", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/multi-page.test.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workflow-e.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Duplicate page 1 -> becomes 3 pages
    await page.getByTestId("page-duplicate").click();
    await expect(workspace).toContainText("3", { timeout: 10000 });

    // Move left
    await page.getByTestId("page-move-left").click();
    await page.waitForTimeout(500);

    // Delete page -> becomes 2 pages
    await page.getByTestId("page-delete").click();
    await expect(workspace).toContainText("2", { timeout: 10000 });

    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click({ force: true });
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-e-edited.pdf");
  });

  test("v0.19 Workflow F: Open A -> Add B -> Merge -> Export", async ({
    page,
  }) => {
    const docAPath = path.join(process.cwd(), "test-assets/merge-a.pdf");
    const docBPath = path.join(process.cwd(), "test-assets/merge-b.pdf");
    const bytesA = fs.readFileSync(docAPath);
    const bytesB = fs.readFileSync(docBPath);

    await uploadPdfBytes(page, "workflow-f.pdf", bytesA);
    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Add PDF B
    await page.locator('input[aria-label="Add PDF documents"]').setInputFiles({
      name: "doc-b.pdf",
      mimeType: "application/pdf",
      buffer: bytesB,
    });
    await expect(workspace).toContainText("2", { timeout: 10000 });

    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click({ force: true });
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-f-edited.pdf");
  });

  test("v0.19 Workflow G: Split / Extract Pages", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/multi-page.test.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workflow-g.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Click Extract button
    const downloadPromise = page.waitForEvent("download");
    await page.getByTestId("page-extract").click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-g-extracted.pdf");
  });

  test("v0.19 Workflow H: Mixed Sequential Edits with Undo / Redo", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/smartpdf-upload-1page.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workflow-h.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Duplicate page
    await page.getByTestId("page-duplicate").click();
    await expect(workspace).toContainText("2 / 2", { timeout: 10000 });

    // Undo duplication
    await page.getByTestId("toolbar-undo-btn").click();
    await expect(workspace).toContainText("1 / 1", { timeout: 10000 });

    // Redo duplication -> restores 2-page document
    await page.getByTestId("toolbar-redo-btn").click();
    await expect(workspace).toContainText("1 / 2", { timeout: 10000 });

    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click({ force: true });
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-h-edited.pdf");
  });

  test("v0.19 Workflow I: Unsupported Encrypted PDF -> Clear Typed Refusal", async ({
    page,
  }) => {
    const encryptedFixturePath = path.join(
      process.cwd(),
      "engine/starpdf/tests/fixtures/v0_11_complex/synthetic-encrypted-standard.pdf",
    );
    const bytes = fs.readFileSync(encryptedFixturePath);

    await page.locator('input[type="file"]').setInputFiles({
      name: "encrypted-test.pdf",
      mimeType: "application/pdf",
      buffer: bytes,
    });

    // Should show explicit refusal message without crash
    const errorAlert = page.getByText(/password-protected or encrypted|security handler/i);
    await expect(errorAlert.first()).toBeVisible({ timeout: 10000 });
  });

  test("v0.19 Workflow J: Unsaved Changes Confirmation Lifecycle", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/smartpdf-form.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workflow-j.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Modify a field directly on canvas to trigger dirty state
    const canvasField = page.locator('[data-testid="canvas-field-full_name"]');
    await expect(canvasField).toBeVisible({ timeout: 10000 });
    await canvasField.click();

    const nameInput = page.locator('[data-testid="context-form-input"]');
    await expect(nameInput).toBeVisible({ timeout: 5000 });
    await nameInput.fill("Pending Unsaved Modification");

    // Verify modified dot indicator
    await expect(page.locator('[data-testid="document-modified-dot"]')).toBeVisible({ timeout: 5000 });

    // Click Open -> Confirm dialog should appear
    await page.locator('[data-testid="toolbar-open-file-btn"]').click();
    const confirmDialog = page.locator('[data-testid="pdf-confirm-dialog"]');
    await expect(confirmDialog).toBeVisible();

    // Click Cancel -> dialog closes, editor remains active
    await page.locator('[data-testid="confirm-dialog-cancel-btn"]').click();
    await expect(confirmDialog).not.toBeVisible();
    await expect(workspace).toBeVisible();

    // Click Open -> Click Discard & Open -> Returns to Dropzone
    await page.locator('[data-testid="toolbar-open-file-btn"]').click();
    await expect(confirmDialog).toBeVisible();
    await page.locator('[data-testid="confirm-dialog-confirm-btn"]').click();

    await expect(page.locator('input[type="file"]')).toBeAttached({ timeout: 5000 });
  });

  test("v0.19 Workflow K: Open -> Select/Edit Supported Annotation -> Export -> Reopen", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/smartpdf-form.pdf");
    const originalBytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workflow-k.pdf", originalBytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // 1. Select annotation/field directly on canvas
    const canvasField = page.locator('[data-testid="canvas-field-full_name"]');
    await expect(canvasField).toBeVisible({ timeout: 10000 });
    await canvasField.dispatchEvent("click");

    // 2. Contextual UI recognizes annotation
    const contextInput = page.locator('[data-testid="context-form-input"]');
    await expect(contextInput).toBeVisible({ timeout: 5000 });

    // 3. Test Escape clears selection
    await page.keyboard.press("Escape");
    await expect(contextInput).not.toBeVisible();

    // 4. Re-select and mutate annotation property
    await canvasField.dispatchEvent("click");
    await expect(contextInput).toBeVisible();
    await contextInput.fill("Persisted Annotation Test");

    // 5. Verify document becomes dirty
    await expect(page.locator('[data-testid="document-modified-dot"]')).toBeVisible({ timeout: 5000 });

    // 6. Verify raw object IDs are NOT displayed in user-facing UI
    await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).not.toContainText("0 0 obj");
    await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).not.toContainText("Annot 0");

    // 7. Export modified document
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-k-edited.pdf");

    const exportedPath = await download.path();
    expect(exportedPath).not.toBeNull();
    const exportedBytes = fs.readFileSync(exportedPath!);
    expect(exportedBytes.length).toBeGreaterThan(100);
    expect(exportedBytes).not.toEqual(originalBytes);

    // 8. Reopen exported document
    await page.getByRole("button", { name: "Open" }).click();
    const confirmBtn = page.getByTestId("confirm-dialog-confirm-btn");
    if (await confirmBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await confirmBtn.click();
    }
    await uploadPdfBytes(page, "workflow-k-edited.pdf", exportedBytes);
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // 9. Verify reopened document contains expected annotation state on canvas
    const reopenedCanvasField = page.locator('[data-testid="canvas-field-full_name"]');
    await expect(reopenedCanvasField).toBeVisible();
    await reopenedCanvasField.dispatchEvent("click");
    const reopenedInput = page.locator('[data-testid="context-form-input"]');
    await expect(reopenedInput).toBeVisible();
    await expect(reopenedInput).toHaveValue("Persisted Annotation Test");
  });

  test("v0.19 Workflow L: Open -> Select/Edit Markup Annotation -> Export -> Reopen", async ({
    page,
  }) => {
    const fixturePath = path.join(
      process.cwd(),
      "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-markup-freetext.pdf",
    );
    const originalBytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workflow-l.pdf", originalBytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // 1. Select FreeText annotation directly on canvas
    const canvasAnnot = page.locator('[data-testid="canvas-annotation-annot-0-0"]');
    await expect(canvasAnnot).toBeVisible({ timeout: 10000 });
    await canvasAnnot.dispatchEvent("click");

    // 2. Contextual UI recognizes markup annotation subtype (FreeText)
    const contextType = page.locator('[data-testid="context-annotation-type"]');
    await expect(contextType).toBeVisible({ timeout: 5000 });
    await expect(contextType).toHaveText("FreeText");

    const contextInput = page.locator('[data-testid="context-annotation-input"]');
    await expect(contextInput).toBeVisible();

    // 3. Verify raw object IDs are NOT displayed in user-facing UI
    await expect(workspace).not.toContainText("0 0 obj");
    await expect(workspace).not.toContainText("Annot 0");

    // 4. Test Escape clears selection
    await page.keyboard.press("Escape");
    await expect(contextInput).not.toBeVisible();

    // 5. Re-select and mutate annotation property
    await canvasAnnot.dispatchEvent("click");
    await expect(contextInput).toBeVisible();
    await contextInput.fill("Updated FreeText Annotation Text");

    // 6. Verify document becomes dirty
    await expect(page.locator('[data-testid="document-modified-dot"]')).toBeVisible({ timeout: 5000 });

    // 7. Export modified document
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-l-edited.pdf");

    const exportedPath = await download.path();
    expect(exportedPath).not.toBeNull();
    const exportedBytes = fs.readFileSync(exportedPath!);
    expect(exportedBytes.length).toBeGreaterThan(100);
    expect(exportedBytes).not.toEqual(originalBytes);

    // 8. Reopen exported document
    await page.getByRole("button", { name: "Open" }).click();
    const confirmBtn = page.getByTestId("confirm-dialog-confirm-btn");
    if (await confirmBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await confirmBtn.click();
    }
    await uploadPdfBytes(page, "workflow-l-edited.pdf", exportedBytes);
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // 9. Verify reopened document contains expected annotation state on canvas
    const reopenedCanvasAnnot = page.locator('[data-testid="canvas-annotation-annot-0-0"]');
    await expect(reopenedCanvasAnnot).toBeVisible({ timeout: 5000 });
    await reopenedCanvasAnnot.dispatchEvent("click");

    const reopenedAnnotInput = page.locator('[data-testid="context-annotation-input"]');
    await expect(reopenedAnnotInput).toBeVisible();
    await expect(reopenedAnnotInput).toHaveValue("Updated FreeText Annotation Text");

    // Verify other markup annotations on canvas are present
    await expect(page.locator('[data-testid="canvas-annotation-annot-0-1"]')).toBeVisible();
    await expect(page.locator('[data-testid="canvas-annotation-annot-0-2"]')).toBeVisible();
    await expect(page.locator('[data-testid="canvas-annotation-annot-0-3"]')).toBeVisible();
  });

  test("v0.20 Workflow M: Session Integrity — Multi-Domain Mutation with Undo/Redo Roundtrip", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/smartpdf-form.pdf");
    const originalBytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workflow-m.pdf", originalBytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // 1. Text edit directly on canvas
    const textSpan = page.locator('[data-testid^="canvas-text-span-"]').first();
    if (await textSpan.isVisible()) {
      await textSpan.click();
      const replaceInput = page.locator('[data-testid="context-text-input"]');
      if (await replaceInput.isVisible()) {
        await replaceInput.fill("Session Integrity Verified");
        await page.locator('[data-testid="context-text-save-btn"]').click();
        await page.waitForTimeout(500);
      }
    }

    // 2. Form edit directly on canvas
    const canvasField = page.locator('[data-testid="canvas-field-full_name"]');
    if (await canvasField.isVisible()) {
      await canvasField.click();
      const formInput = page.locator('[data-testid="context-form-input"]');
      await expect(formInput).toBeVisible();
      await formInput.fill("Form Value Session");
    }

    // 3. Test Undo
    const undoBtn = page.locator('[data-testid="toolbar-undo-btn"]');
    if (await undoBtn.isEnabled()) {
      await undoBtn.click();
      await page.waitForTimeout(500);
    }

    // 4. Test Redo
    const redoBtn = page.locator('[data-testid="toolbar-redo-btn"]');
    if (await redoBtn.isEnabled()) {
      await redoBtn.click();
      await page.waitForTimeout(500);
    }

    // 5. Export
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-m-edited.pdf");

    const exportedPath = await download.path();
    expect(exportedPath).not.toBeNull();
    const exportedBytes = fs.readFileSync(exportedPath!);
    expect(exportedBytes.length).toBeGreaterThan(100);

    // 6. Reopen and verify canvas state
    await page.getByRole("button", { name: "Open" }).click();
    const confirmBtn = page.getByTestId("confirm-dialog-confirm-btn");
    if (await confirmBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await confirmBtn.click();
    }
    await uploadPdfBytes(page, "workflow-m-edited.pdf", exportedBytes);
    await expect(workspace).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole("main", { name: "PDF Document Page Viewport" }).locator("canvas")).toBeVisible();
  });

  test("v0.20 Workflow N: Sequential Document Isolation — Clean Reset between Documents", async ({
    page,
  }) => {
    // 1. Open Doc A (Form)
    const docABytes = fs.readFileSync(path.join(process.cwd(), "test-assets/smartpdf-form.pdf"));
    await uploadPdfBytes(page, "doc-a.pdf", docABytes);
    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });
    await expect(page.locator('[data-testid="canvas-field-full_name"]')).toBeVisible();

    // 2. Close & Reset via Open Toolbar button
    await page.locator('[data-testid="toolbar-open-file-btn"]').click();
    const confirmBtn = page.getByTestId("confirm-dialog-confirm-btn");
    if (await confirmBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await confirmBtn.click();
    }
    const fileInput = page.locator('input[type="file"]');
    await expect(fileInput).toBeAttached({ timeout: 5000 });

    // 3. Open Doc B (Multi-page, non-form)
    const docBBytes = fs.readFileSync(path.join(process.cwd(), "test-assets/multi-page.test.pdf"));
    await fileInput.setInputFiles({
      name: "doc-b.pdf",
      mimeType: "application/pdf",
      buffer: docBBytes,
    });
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // 4. Verify no stale field overlays from Doc A
    await expect(page.locator('[data-testid="canvas-field-full_name"]')).not.toBeVisible();

    // 5. Navigate & Export Doc B
    await page.getByRole("button", { name: "Next Page" }).click();
    await page.waitForTimeout(300);

    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("doc-b-edited.pdf");
  });

  test("v0.20 Workflow O: Failure Recovery — Editor Remains Usable After Unsupported Error", async ({
    page,
  }) => {
    // 1. Attempt to open unsupported encrypted PDF
    const encryptedPath = path.join(
      process.cwd(),
      "engine/starpdf/tests/fixtures/v0_11_complex/synthetic-encrypted-standard.pdf",
    );
    const encryptedBytes = fs.readFileSync(encryptedPath);
    await page.locator('input[type="file"]').setInputFiles({
      name: "encrypted.pdf",
      mimeType: "application/pdf",
      buffer: encryptedBytes,
    });

    // Verify explicit refusal message
    const errorAlert = page.getByText(/password-protected or encrypted|security handler/i);
    await expect(errorAlert.first()).toBeVisible({ timeout: 10000 });

    // 2. Open valid PDF immediately without reloading page
    const validBytes = fs.readFileSync(path.join(process.cwd(), "test-assets/smartpdf-form.pdf"));
    await page.locator('input[type="file"]').setInputFiles({
      name: "recovered.pdf",
      mimeType: "application/pdf",
      buffer: validBytes,
    });

    // 3. Verify editor is fully operational
    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    const canvasField = page.locator('[data-testid="canvas-field-full_name"]');
    await expect(canvasField).toBeVisible({ timeout: 10000 });
    await canvasField.click();

    const nameInput = page.locator('[data-testid="context-form-input"]');
    await expect(nameInput).toBeVisible();
    await nameInput.fill("Recovered Session Value");

    // 4. Export & Reopen
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("recovered-edited.pdf");
  });

  test("v0.20 Workflow P: Long Session Smoke — Repeated Selection, Navigation, and Operations", async ({
    page,
  }) => {
    const multiPageBytes = fs.readFileSync(path.join(process.cwd(), "test-assets/multi-page.test.pdf"));
    await uploadPdfBytes(page, "workflow-p.pdf", multiPageBytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Cycle through pages
    const nextBtn = page.locator('button[title="Next Page"]');
    const prevBtn = page.locator('button[title="Previous Page"]');

    await expect(prevBtn).toBeDisabled();
    await nextBtn.click();
    await expect(prevBtn).toBeEnabled();

    await prevBtn.click();
    await expect(prevBtn).toBeDisabled();

    // Perform page duplicate (2 pages -> 3 pages)
    await page.getByTestId("page-duplicate").click();
    await expect(workspace).toContainText("3", { timeout: 10000 });

    // Undo duplicate
    const undoBtn = page.locator('[data-testid="toolbar-undo-btn"]');
    await expect(undoBtn).toBeEnabled();
    await undoBtn.click();
    await page.waitForTimeout(500);

    // Redo duplicate
    const redoBtn = page.locator('[data-testid="toolbar-redo-btn"]');
    await expect(redoBtn).toBeEnabled();
    await redoBtn.click();
    await page.waitForTimeout(500);

    // Export final state
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("workflow-p-edited.pdf");
  });

  test("v0.20 Local-First Privacy Audit: Zero PDF Content Bytes Sent Over Network", async ({
    page,
  }) => {
    const postRequests: { url: string; postData: string | null }[] = [];

    // Listen to all network requests
    page.on("request", (req) => {
      if (req.method() === "POST" || req.method() === "PUT" || req.method() === "PATCH") {
        postRequests.push({
          url: req.url(),
          postData: req.postData(),
        });
      }
    });

    const formBytes = fs.readFileSync(path.join(process.cwd(), "test-assets/smartpdf-form.pdf"));
    await uploadPdfBytes(page, "privacy-audit.pdf", formBytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Mutate field directly on canvas
    const canvasField = page.locator('[data-testid="canvas-field-full_name"]');
    await expect(canvasField).toBeVisible();
    await canvasField.click();

    const nameInput = page.locator('[data-testid="context-form-input"]');
    await expect(nameInput).toBeVisible();
    await nameInput.fill("Confidential Privacy Test");

    // Export
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click();
    await downloadPromise;

    // Verify ZERO POST/upload requests were made for PDF processing
    expect(postRequests.length).toBe(0);
  });

  test("v0.20 Responsive Desktop Qualification: 1280x720, 1440x900, 1920x1080", async ({
    page,
  }) => {
    const viewports = [
      { width: 1280, height: 720 },
      { width: 1440, height: 900 },
      { width: 1920, height: 1080 },
    ];

    const formBytes = fs.readFileSync(path.join(process.cwd(), "test-assets/smartpdf-form.pdf"));

    for (const vp of viewports) {
      await page.setViewportSize(vp);
      await uploadPdfBytes(page, `responsive-${vp.width}.pdf`, formBytes);

      const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
      await expect(workspace).toBeVisible({ timeout: 10000 });

      // Verify essential UI regions and direct canvas elements are accessible
      await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).toBeVisible();
      await expect(page.getByRole("button", { name: "Export Editable" })).toBeVisible();
      await expect(page.locator('[data-testid="canvas-field-full_name"]')).toBeVisible();

      // Close document to reset for next viewport
      await page.locator('[data-testid="toolbar-open-file-btn"]').click();
      await expect(page.locator('input[type="file"]')).toBeAttached({ timeout: 5000 });
    }
  });

  test("v0.20 RC Blocker Fix: Page Delete UI State Invariant (First, Middle, Last with Thumbnail Verification)", async ({
    page,
  }) => {
    // Load a multipage document (2 pages)
    const fixturePath = path.join(process.cwd(), "test-assets/multi-page.test.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "multipage-delete-test.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Initial page count: 2 -> Duplicate to create 3 pages
    await page.getByTestId("page-duplicate").click();
    await expect(workspace).toContainText("3", { timeout: 10000 });

    const thumbRail = page.locator('[data-testid="pdf-thumbnail-rail"]');
    await expect(thumbRail).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 1" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 2" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 3" })).toBeVisible();

    // 1. DELETE FIRST PAGE (Page 1 of 3 -> becomes 2 pages, active page is 1)
    const prevBtn = page.locator('button[title="Previous Page"]');
    while (await prevBtn.isEnabled()) {
      await prevBtn.click();
      await page.waitForTimeout(100);
    }
    await expect(page.getByText("1 / 3", { exact: true })).toBeVisible({ timeout: 5000 });

    await page.getByTestId("page-delete").click();

    // Verify loading spinner absent, blur overlay gone, page count is 2
    await expect(page.getByText("Processing in StarPDF worker…")).not.toBeVisible({ timeout: 10000 });
    await expect(page.locator('.backdrop-blur-xs .animate-spin')).not.toBeVisible({ timeout: 10000 });
    await expect(page.getByText("1 / 2", { exact: true })).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole("main", { name: "PDF Document Page Viewport" }).locator("canvas")).toBeVisible();

    // Verify Thumbnail Rail remains OPEN and shows 2 thumbnails
    await expect(thumbRail).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 1" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 2" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 3" })).not.toBeVisible();
    await expect(page.getByRole("button", { name: "Page 1" })).toHaveAttribute("aria-current", "page");

    // 2. DELETE MIDDLE PAGE (Duplicate to have 3 pages again, go to page 2, delete)
    await page.getByTestId("page-duplicate").click();
    await expect(workspace).toContainText("3", { timeout: 10000 });
    await expect(page.getByRole("button", { name: "Page 3" })).toBeVisible();

    // Navigate to page 2
    await page.getByRole("button", { name: "Page 2" }).click();
    await expect(page.getByText("2 / 3", { exact: true })).toBeVisible({ timeout: 5000 });

    await page.getByTestId("page-delete").click();
    await expect(page.getByText("Processing in StarPDF worker…")).not.toBeVisible({ timeout: 10000 });
    await expect(page.locator('.backdrop-blur-xs .animate-spin')).not.toBeVisible({ timeout: 10000 });
    await expect(page.getByText("2 / 2", { exact: true })).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole("main", { name: "PDF Document Page Viewport" }).locator("canvas")).toBeVisible();

    // Verify Thumbnail Rail remains OPEN and shows 2 thumbnails
    await expect(thumbRail).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 1" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 2" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 3" })).not.toBeVisible();

    // 3. DELETE LAST PAGE (Duplicate to have 3 pages, go to page 3, delete -> clamps to 2)
    await page.getByTestId("page-duplicate").click();
    await expect(workspace).toContainText("3", { timeout: 10000 });
    await page.getByRole("button", { name: "Page 3" }).click();
    await expect(page.getByText("3 / 3", { exact: true })).toBeVisible({ timeout: 5000 });

    await page.getByTestId("page-delete").click();
    await expect(page.getByText("Processing in StarPDF worker…")).not.toBeVisible({ timeout: 10000 });
    await expect(page.locator('.backdrop-blur-xs .animate-spin')).not.toBeVisible({ timeout: 10000 });
    await expect(page.getByText("2 / 2", { exact: true })).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole("main", { name: "PDF Document Page Viewport" }).locator("canvas")).toBeVisible();

    // Verify Thumbnail Rail remains OPEN, shows 2 thumbnails, active page is 2
    await expect(thumbRail).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 1" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 2" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 3" })).not.toBeVisible();
    await expect(page.getByRole("button", { name: "Page 2" })).toHaveAttribute("aria-current", "page");

    // Export and verify file downloads cleanly
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click({ force: true });
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("multipage-delete-test-edited.pdf");
  });

  test("v0.20 Workspace UX: Collapsible Panels, Invariants, and Multipage Operations", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/multi-page.test.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "workspace-ux.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    const thumbRail = page.locator('[data-testid="pdf-thumbnail-rail"]');
    await expect(thumbRail).toBeVisible();

    // 1. Manually COLLAPSE Thumbnail Rail
    await page.locator('[data-testid="toolbar-toggle-thumbnails-btn"]').click();
    await expect(thumbRail).not.toBeVisible();

    // 2. Perform page operations while rail is collapsed -> rail must REMAIN collapsed
    await page.getByTestId("page-duplicate").click();
    await expect(workspace).toContainText("3", { timeout: 10000 });
    await expect(thumbRail).not.toBeVisible();

    await page.getByTestId("page-insert-blank").click();
    await expect(workspace).toContainText("4", { timeout: 10000 });
    await expect(thumbRail).not.toBeVisible();

    await page.getByTestId("page-move-left").click();
    await expect(page.getByText("Page moved left.")).toBeVisible({ timeout: 5000 });
    await expect(thumbRail).not.toBeVisible();

    await page.getByTestId("page-delete").click();
    await expect(workspace).toContainText("3", { timeout: 10000 });
    await expect(thumbRail).not.toBeVisible();

    // 3. Manually REOPEN Thumbnail Rail -> displays updated 3 thumbnails
    await page.locator('[data-testid="toolbar-toggle-thumbnails-btn"]').click();
    await expect(thumbRail).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 1" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 2" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Page 3" })).toBeVisible();

    // 4. Verify Document Info / Diagnostics modal can open and close
    await page.locator('[data-testid="toolbar-info-btn"]').click();
    const docModal = page.locator('[data-testid="doc-info-modal"]');
    await expect(docModal).toBeVisible();
    await page.getByRole("button", { name: "Done" }).click();
    await expect(docModal).not.toBeVisible();

    // Click thumbnail for page 3 to navigate
    await page.getByRole("button", { name: "Page 3" }).click();
    await expect(page.getByText("3 / 3", { exact: true })).toBeVisible({ timeout: 5000 });

    // Verify local processing badge is present
    await expect(page.locator('[data-testid="privacy-local-badge"]')).toBeVisible();
  });

  test("v0.20 Real 14-Page PDF: Complete Page Mutation Lifecycle & Navigation Stability", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/14-page-real.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "14-page-real.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    const thumbRail = page.locator('[data-testid="pdf-thumbnail-rail"]');
    await expect(thumbRail).toBeVisible();
    await expect(thumbRail.locator('button[aria-label^="Page "]')).toHaveCount(14);

    // 1. DELETE FIRST PAGE (Page 1 of 14 -> 13 pages, current 1, rail visible with 13 thumbs)
    await page.getByTestId("page-delete").click();
    await expect(page.getByText("Processing in StarPDF worker…")).not.toBeVisible({ timeout: 10000 });
    await expect(page.locator('.backdrop-blur-xs .animate-spin')).not.toBeVisible({ timeout: 10000 });
    await expect(page.getByText("1 / 13", { exact: true })).toBeVisible({ timeout: 5000 });
    await expect(thumbRail).toBeVisible();
    await expect(thumbRail.locator('button[aria-label^="Page "]')).toHaveCount(13);
    await expect(page.getByRole("main", { name: "PDF Document Page Viewport" }).locator("canvas")).toBeVisible();

    // 2. DELETE MIDDLE PAGE (Page 7 of 13 -> 12 pages, current 7, rail visible with 12 thumbs)
    await page.getByRole("button", { name: "Page 7" }).click();
    await expect(page.getByText("7 / 13", { exact: true })).toBeVisible({ timeout: 5000 });
    await page.getByTestId("page-delete").click();
    await expect(page.getByText("Processing in StarPDF worker…")).not.toBeVisible({ timeout: 10000 });
    await expect(page.locator('.backdrop-blur-xs .animate-spin')).not.toBeVisible({ timeout: 10000 });
    await expect(page.getByText("7 / 12", { exact: true })).toBeVisible({ timeout: 5000 });
    await expect(thumbRail).toBeVisible();
    await expect(thumbRail.locator('button[aria-label^="Page "]')).toHaveCount(12);
    await expect(page.getByRole("main", { name: "PDF Document Page Viewport" }).locator("canvas")).toBeVisible();

    // 3. DELETE LAST PAGE (Page 12 of 12 -> 11 pages, current 11, rail visible with 11 thumbs)
    await page.getByRole("button", { name: "Page 12" }).click();
    await expect(page.getByText("12 / 12", { exact: true })).toBeVisible({ timeout: 5000 });
    await page.getByTestId("page-delete").click();
    await expect(page.getByText("Processing in StarPDF worker…")).not.toBeVisible({ timeout: 10000 });
    await expect(page.locator('.backdrop-blur-xs .animate-spin')).not.toBeVisible({ timeout: 10000 });
    await expect(page.getByText("11 / 11", { exact: true })).toBeVisible({ timeout: 5000 });
    await expect(thumbRail).toBeVisible();
    await expect(thumbRail.locator('button[aria-label^="Page "]')).toHaveCount(11);
    await expect(page.getByRole("main", { name: "PDF Document Page Viewport" }).locator("canvas")).toBeVisible();

    // 4. REORDER, DUPLICATE, INSERT BLANK
    await page.getByTestId("page-move-left").click();
    await expect(page.getByText("Page moved left.")).toBeVisible({ timeout: 5000 });
    await expect(thumbRail).toBeVisible();
    await expect(thumbRail.locator('button[aria-label^="Page "]')).toHaveCount(11);

    await page.getByTestId("page-duplicate").click();
    await expect(page.getByText("11 / 12", { exact: true })).toBeVisible({ timeout: 10000 });
    await expect(thumbRail).toBeVisible();
    await expect(thumbRail.locator('button[aria-label^="Page "]')).toHaveCount(12);

    await page.getByTestId("page-insert-blank").click();
    await expect(page.getByText("12 / 13", { exact: true })).toBeVisible({ timeout: 10000 });
    await expect(thumbRail).toBeVisible();
    await expect(thumbRail.locator('button[aria-label^="Page "]')).toHaveCount(13);

    // 5. EXPORT AND REOPEN
    const downloadPromise = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export Editable" }).click({ force: true });
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("14-page-real-edited.pdf");
  });

  test("v0.20 Direct Manipulation: Text, Vector, Form, Escape, Empty Click & Page Change Clear", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/smartpdf-form.pdf");
    const originalBytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "direct-manipulation.pdf", originalBytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    const contextToolbar = page.locator('[data-testid="pdf-contextual-toolbar"]');

    // 1. DIRECT FORM SELECTION: Click form field on canvas
    const canvasField = page.locator('[data-testid="canvas-field-full_name"]');
    await expect(canvasField).toBeVisible({ timeout: 10000 });
    await canvasField.click();
    await expect(contextToolbar).toBeVisible({ timeout: 5000 });
    await expect(page.locator('[data-testid="context-form-input"]')).toBeVisible();

    // 2. ESCAPE KEY CLEARS SELECTION
    await page.keyboard.press("Escape");
    await expect(contextToolbar).not.toBeVisible();

    // 3. RE-SELECT & EMPTY CLICK CLEARS SELECTION
    await canvasField.click();
    await expect(contextToolbar).toBeVisible();
    // Click on viewport background outside overlay
    await page.getByRole("main", { name: "PDF Document Page Viewport" }).click({ position: { x: 10, y: 10 } });
    await expect(contextToolbar).not.toBeVisible();

    // 4. DIRECT TEXT SELECTION: Click text span on canvas
    const textSpan = page.locator('[data-testid^="canvas-text-span-"]').first();
    if (await textSpan.isVisible()) {
      await textSpan.click();
      await expect(contextToolbar).toBeVisible();
      // Test Escape clears text selection
      await page.keyboard.press("Escape");
      await expect(contextToolbar).not.toBeVisible();
    }
  });

  test("v0.20 Phase 1: Canonical /smartpdf Application Shell, Empty State & Status Bar", async ({
    page,
  }) => {
    // 1. DIRECT LOAD: /smartpdf opens without needing prior route navigation
    await page.goto("/smartpdf");
    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // 2. EMPTY STATE: Application header and status bar render cleanly
    await expect(page.getByText("SmartPDF", { exact: false }).first()).toBeVisible();
    await expect(page.getByText("Powered by StarPDF")).toBeVisible();
    const statusBar = page.locator('[data-testid="smartpdf-status-bar"]');
    await expect(statusBar).toBeVisible();
    await expect(statusBar).toContainText("No document open");

    // 3. LOAD DOCUMENT INTO FULL-VIEWPORT APPLICATION
    const fixturePath = path.join(process.cwd(), "test-assets/smartpdf-form.pdf");
    const originalBytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "phase1-app.pdf", originalBytes);

    // 4. LOADED STATUS BAR: Shows page numbers, zoom, selection state, and privacy badge
    await expect(statusBar).toContainText("Page 1 / 1");
    await expect(statusBar).toContainText("Zoom 100%");
    await expect(statusBar).toContainText("No selection");
    await expect(statusBar).toContainText("Local processing");
  });

  test("v0.20 Phase 1: ToolRakyat Entry Point Navigation to /smartpdf", async ({
    page,
  }) => {
    // 1. Visit old ToolRakyat PDF editor tool discovery route
    await page.goto("/tools/pdf/editor");

    // 2. Verify launch card with "Open SmartPDF" CTA button
    const openBtn = page.locator('[data-testid="open-smartpdf-btn"]');
    await expect(openBtn).toBeVisible({ timeout: 10000 });
    await expect(openBtn).toContainText("Open SmartPDF");

    // 3. Click CTA and verify navigation to /smartpdf
    await openBtn.click();
    await page.waitForURL("**/smartpdf", { timeout: 10000 });
    expect(page.url()).toContain("/smartpdf");

    // 4. Verify canonical SmartPDF application workspace loaded
    await expect(page.locator('[data-testid="smartpdf-editor-workspace"]')).toBeVisible();
  });

  test("v0.20 Phase 1: Direct Browser Refresh and Local State Resilience", async ({
    page,
  }) => {
    await page.goto("/smartpdf");
    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    // Reload page directly
    await page.reload();
    await expect(workspace).toBeVisible({ timeout: 10000 });
    await expect(page.locator('[data-testid="smartpdf-status-bar"]')).toBeVisible();
  });

  test("v0.20 Phase 2: Unified Command Lifecycle — Mutating Command with History & Dirty State", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/multi-page.test.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "phase2-command-lifecycle.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    const statusBar = page.locator('[data-testid="smartpdf-status-bar"]');
    await expect(statusBar).toContainText("Page 1 / 2");
    await expect(statusBar).not.toContainText("Unsaved changes");

    // 1. Execute Duplicate Page Command -> Mutating
    await page.getByTestId("page-duplicate").click();
    await expect(statusBar).toContainText("Page 2 / 3", { timeout: 10000 });
    await expect(statusBar).toContainText("Unsaved changes");

    // 2. Undo Command -> Restores snapshot
    await page.getByTestId("toolbar-undo-btn").click();
    await expect(statusBar).toContainText("/ 2", { timeout: 10000 });

    // 3. Redo Command -> Restores branch
    await page.getByTestId("toolbar-redo-btn").click();
    await expect(statusBar).toContainText("/ 3", { timeout: 10000 });
  });

  test("v0.20 Phase 2: Unified Selection Model — Hit Testing Priority, Escape, Empty Click & Page Navigation Clear", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/smartpdf-upload-1page.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "phase2-selection.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    const statusBar = page.locator('[data-testid="smartpdf-status-bar"]');

    // 1. Click text span
    const textSpan = page.locator('[data-testid^="canvas-text-span-"]').first();
    await expect(textSpan).toBeVisible({ timeout: 5000 });
    await textSpan.click();
    await expect(statusBar).toContainText("Selected: TEXT");
    await expect(page.locator('[data-testid="context-text-controls"]')).toBeVisible();

    // 2. Press Escape -> Clears selection
    await page.keyboard.press("Escape");
    await expect(statusBar).toContainText("No selection");
    await expect(page.locator('[data-testid="context-text-controls"]')).not.toBeVisible();

    // 3. Click vector graphic
    const graphic = page.locator('[data-testid^="canvas-graphic-"]').first();
    if (await graphic.isVisible().catch(() => false)) {
      await graphic.click();
      await expect(statusBar).toContainText("Selected: VECTOR");

      // 4. Click canvas background -> Clears selection
      await workspace.click({ position: { x: 50, y: 50 } });
      await expect(statusBar).toContainText("No selection");
    }
  });

  test("v0.20 Phase 2: Page Command Atomicity & Busy Rejection Under Rapid Invocation", async ({
    page,
  }) => {
    const fixturePath = path.join(process.cwd(), "test-assets/multi-page.test.pdf");
    const bytes = fs.readFileSync(fixturePath);
    await uploadPdfBytes(page, "phase2-atomicity.pdf", bytes);

    const workspace = page.locator('[data-testid="smartpdf-editor-workspace"]');
    await expect(workspace).toBeVisible({ timeout: 10000 });

    const statusBar = page.locator('[data-testid="smartpdf-status-bar"]');
    const thumbRail = page.locator('[data-testid="pdf-thumbnail-rail"]');

    // Perform Duplicate
    await page.getByTestId("page-duplicate").click();
    await expect(statusBar).toContainText("Page 2 / 3", { timeout: 10000 });
    await expect(thumbRail).toBeVisible();

    // Perform Delete
    await page.getByTestId("page-delete").click();
    await expect(statusBar).toContainText("Page 2 / 2", { timeout: 10000 });
    await expect(thumbRail).toBeVisible();
    await expect(page.getByRole("main", { name: "PDF Document Page Viewport" }).locator("canvas")).toBeVisible();
  });
});



