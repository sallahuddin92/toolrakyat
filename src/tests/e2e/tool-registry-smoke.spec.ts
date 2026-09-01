import { expect, test, type Page } from "@playwright/test";
import { PDFDocument, StandardFonts } from "pdf-lib";
import JSZip from "jszip";

import { tools } from "@/lib/tools/registry";
import { getImplementationKey } from "@/lib/tools/implementation-registry";

const PNG = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAFElEQVR42mNk+M/wn4GBgYGJAQoAHgQCAf2Q4hAAAAAASUVORK5CYII=", "base64");

async function pdfFixture(): Promise<Buffer> {
  const document = await PDFDocument.create();
  const font = await document.embedFont(StandardFonts.Helvetica);
  for (const label of ["First page", "Second page"]) {
    const page = document.addPage([320, 480]);
    page.drawText(label, { x: 40, y: 400, size: 18, font });
  }
  return Buffer.from(await document.save({ useObjectStreams: false }));
}

async function outputBytes(page: Page): Promise<number[]> {
  const link = page.locator('a[download]').first();
  await expect(link).toBeVisible({ timeout: 30_000 });
  return link.evaluate(async (anchor) => {
    const response = await fetch((anchor as HTMLAnchorElement).href);
    return Array.from(new Uint8Array(await response.arrayBuffer()));
  });
}

const sampleInput: Record<string, string> = {
  "text-password-generator": "20", "dev-uuid-generator": "2", "converter-csv-to-json": "name,amount\nAli,25",
  "converter-json-to-csv": '[{"name":"Ali","amount":25}]', "converter-markdown-to-html": "# Hello",
  "compression-json-minifier": '{ "a": 1 }', "compression-css-minifier": ".a { color: red; }",
  "compression-html-minifier": "<p> Hello </p>", "dev-json-formatter": '{"a":1}', "dev-json-validator": "[]",
  "calc-loan": "100000,4,20", "calc-profit-margin": "60,100", "calc-discount": "100,20", "calc-sst": "100,8",
  "calc-age": "1990-01-01", "calc-date-difference": "2026-01-01 2026-02-01", "calc-bmi": "70,175",
  "calc-compound-interest": "1000,5,2,12", "dev-color-converter": "#0EA5E9", "qr-whatsapp-link-generator": "+60123456789",
};

test.describe("registered tool restoration", () => {
  test("every registered route resolves and every implemented route has an executable mapping", async ({ request }) => {
    test.setTimeout(240_000);
    for (const tool of tools) {
      await test.step(tool.id, async () => {
        const response = await request.get(tool.route);
        expect(response.status(), `${tool.route} status`).toBe(200);
        if (tool.isImplemented) expect(getImplementationKey(tool), `${tool.id} mapping`).toBeDefined();
        expect(await response.text()).not.toContain("Tool wiring pending");
      });
    }
  });

  test("every lightweight generic utility completes its primary happy path", async ({ page }) => {
    test.setTimeout(360_000);
    const candidates = tools.filter((tool) => getImplementationKey(tool) === "generic-utility" && !["pdf", "image"].includes(tool.categoryId) && !["zip-create", "zip-extract", "compression-batch-image-compress"].includes(tool.id));
    for (const tool of candidates) {
      await test.step(tool.id, async () => {
        await page.goto(tool.route);
        await expect(page.getByTestId("utility-tool")).toHaveAttribute("data-tool-id", tool.id);
        const input = page.getByTestId("tool-text-input");
        await input.fill(sampleInput[tool.id] ?? "Rakyat سلام 中文");
        await page.getByTestId("tool-primary-action").click();
        await expect(page.getByTestId("tool-result").or(page.locator('a[download]')).first()).toBeVisible({ timeout: 30_000 });
        await expect(page.getByText("Something went wrong")).toHaveCount(0);
      });
    }
  });

  test("every image and archive utility emits a decodable non-empty output", async ({ page }) => {
    test.setTimeout(240_000);
    const archive = new JSZip(); archive.file("hello.txt", "hello");
    const zip = Buffer.from(await archive.generateAsync({ type: "uint8array" }));
    for (const tool of tools.filter((candidate) => getImplementationKey(candidate) === "generic-utility" && (candidate.categoryId === "image" || ["zip-create", "zip-extract", "compression-batch-image-compress"].includes(candidate.id)))) {
      await test.step(tool.id, async () => {
        await page.goto(tool.route);
        const input = page.getByTestId("tool-file-input");
        if (tool.id === "zip-extract") await input.setInputFiles({ name: "fixture.zip", mimeType: "application/zip", buffer: zip });
        else if (tool.id === "zip-create") await input.setInputFiles([{ name: "one.txt", mimeType: "text/plain", buffer: Buffer.from("one") }, { name: "two.txt", mimeType: "text/plain", buffer: Buffer.from("two") }]);
        else await input.setInputFiles({ name: "pixel.png", mimeType: "image/png", buffer: PNG });
        await page.getByTestId("tool-primary-action").click();
        const output = await outputBytes(page);
        expect(output.length, `${tool.id} bytes`).toBeGreaterThan(8);
        if (tool.categoryId === "image") {
          const png = output.slice(1, 4).join(",") === "80,78,71";
          const jpeg = output.slice(0, 3).join(",") === "255,216,255";
          const webp = String.fromCharCode(...output.slice(0, 4)) === "RIFF";
          expect(png || jpeg || webp, `${tool.id} image signature`).toBe(true);
        }
        else expect(output.slice(0, 2)).toEqual([80, 75]);
      });
    }
  });

  test("every simple PDF utility emits a valid reopenable PDF or ZIP", async ({ page }) => {
    test.setTimeout(360_000);
    const pdf = await pdfFixture();
    for (const tool of tools.filter((candidate) => getImplementationKey(candidate) === "generic-utility" && candidate.categoryId === "pdf")) {
      await test.step(tool.id, async () => {
        await page.goto(tool.route);
        const input = page.getByTestId("tool-file-input");
        if (tool.id === "pdf-images-to-pdf") await input.setInputFiles({ name: "pixel.png", mimeType: "image/png", buffer: PNG });
        else if (tool.id === "pdf-merge") await input.setInputFiles([{ name: "one.pdf", mimeType: "application/pdf", buffer: pdf }, { name: "two.pdf", mimeType: "application/pdf", buffer: pdf }]);
        else if (tool.id === "pdf-sign") await input.setInputFiles([{ name: "fixture.pdf", mimeType: "application/pdf", buffer: pdf }, { name: "signature.png", mimeType: "image/png", buffer: PNG }]);
        else await input.setInputFiles({ name: "fixture.pdf", mimeType: "application/pdf", buffer: pdf });
        await page.getByTestId("tool-primary-action").click();
        const output = await outputBytes(page);
        expect(output.length, `${tool.id} bytes`).toBeGreaterThan(100);
        if (tool.id === "pdf-split") expect(output.slice(0, 2)).toEqual([80, 75]);
        else {
          expect(String.fromCharCode(...output.slice(0, 5))).toBe("%PDF-");
          await PDFDocument.load(Uint8Array.from(output));
        }
      });
    }
  });

  test("non-PDF utility routes do not load StarPDF, PDF.js, workers, or adaptive fonts", async ({ page }) => {
    const requested: string[] = [];
    page.on("request", (request) => requested.push(request.url()));
    await page.goto("/tools/text/slug-generator");
    await page.getByTestId("tool-text-input").fill("Lightweight route");
    await page.getByTestId("tool-primary-action").click();
    await expect(page.getByTestId("tool-result")).toContainText("lightweight-route");
    await page.goto("/tools/image/resize");
    await page.getByTestId("tool-file-input").setInputFiles({ name: "pixel.png", mimeType: "image/png", buffer: PNG });
    await page.getByTestId("tool-primary-action").click();
    await expect(page.locator('a[download]')).toBeVisible();
    expect(requested.filter((url) => /starpdf|pdf\.worker|starpdf\.worker|\/fonts\/Noto/i.test(url))).toEqual([]);
  });
});
