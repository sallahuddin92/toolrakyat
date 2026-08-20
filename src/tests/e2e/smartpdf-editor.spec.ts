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
  const beforePixels = await sharp(before).extract(region).raw().toBuffer();
  const afterPixels = await sharp(after).extract(region).raw().toBuffer();
  let changedChannels = 0;
  for (let index = 0; index < beforePixels.length; index += 1) {
    if (Math.abs(beforePixels[index] - afterPixels[index]) > 8) changedChannels += 1;
  }
  return changedChannels / beforePixels.length;
}

test.describe("SmartPDF — Advanced PDF Editor", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tools/pdf/editor");
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
    const canvas = await uploadPdfBytes(page, "page-operations-extracted.pdf", extracted);
    await expect(page.getByText("1 / 1", { exact: true })).toBeVisible();
    expect((await canvas.screenshot()).byteLength).toBeGreaterThan(1_000);
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
    const metadata = await sharp(originalPng).metadata();
    const region = {
      left: 45,
      top: 125,
      width: Math.min(320, (metadata.width ?? 0) - 45),
      height: 60,
    };
    expect(region.width).toBeGreaterThan(300);
    const originalRegion = await sharp(originalPng).extract(region).raw().toBuffer();
    const mutatedRegion = await sharp(mutatedPng).extract(region).raw().toBuffer();
    let changedChannels = 0;
    for (let index = 0; index < originalRegion.length; index += 1) {
      if (Math.abs(originalRegion[index] - mutatedRegion[index]) > 8) changedChannels += 1;
    }
    expect(changedChannels / originalRegion.length).toBeGreaterThan(0.005);
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
    expect(
      await changedRegionRatio(originalPng, changedPng, {
        left: 40,
        top: 245,
        width: 260,
        height: 100,
      }),
    ).toBeGreaterThan(0.001);
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
        left: 180,
        top: 70,
        width: 90,
        height: Math.min(290, (metadata.height ?? 0) - 70),
      }),
    ).toBeGreaterThan(0.001);
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
    ).toBeGreaterThan(0.001);
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
    await expect(page.getByTestId("smartpdf-editor-workspace")).toHaveCount(0);
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
});
