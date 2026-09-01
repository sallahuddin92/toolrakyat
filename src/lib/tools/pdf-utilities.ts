import type { PDFDocument as PDFDocumentType } from "pdf-lib";

export type UtilityOutput = { blob: Blob; fileName: string };

function parsePages(value: string, pageCount: number): number[] {
  const pages = new Set<number>();
  for (const token of value.split(",").map((part) => part.trim()).filter(Boolean)) {
    const match = token.match(/^(\d+)(?:-(\d+))?$/);
    if (!match) throw new Error("Use page numbers or ranges such as 1-3,5.");
    const start = Number(match[1]);
    const end = Number(match[2] ?? match[1]);
    if (start < 1 || end < start || end > pageCount) throw new Error(`Page range must be within 1-${pageCount}.`);
    for (let page = start; page <= end; page += 1) pages.add(page - 1);
  }
  if (!pages.size) throw new Error("Select at least one page.");
  return [...pages];
}

async function bytes(file: File): Promise<Uint8Array> {
  return new Uint8Array(await file.arrayBuffer());
}

async function validatePdf(output: Uint8Array): Promise<void> {
  const { PDFDocument } = await import("pdf-lib");
  const document = await PDFDocument.load(output, { ignoreEncryption: false });
  if (document.getPageCount() < 1) throw new Error("Generated PDF has no pages.");
}

function pdfOutput(output: Uint8Array, fileName: string): UtilityOutput {
  return { blob: new Blob([output as BlobPart], { type: "application/pdf" }), fileName };
}

async function loadPdfLib(file: File): Promise<{ document: PDFDocumentType; PDFDocument: typeof import("pdf-lib").PDFDocument }> {
  const { PDFDocument } = await import("pdf-lib");
  return { document: await PDFDocument.load(await file.arrayBuffer()), PDFDocument };
}

export async function processPdfUtility(toolId: string, files: File[], option: string): Promise<UtilityOutput> {
  if (!files.length) throw new Error("Choose at least one input file.");

  if (toolId === "pdf-merge") {
    if (files.length < 2) throw new Error("Choose at least two PDF files.");
    const { mergeStarPdfDocuments } = await import("@/lib/pdf/starpdf-page-worker-client");
    const output = await mergeStarPdfDocuments(await Promise.all(files.map(bytes)));
    await validatePdf(output);
    return pdfOutput(output, "merged.pdf");
  }

  if (toolId === "pdf-split") {
    const [{ StarPdfClient }, { splitStarPdfDocument }, JSZip] = await Promise.all([
      import("@/lib/pdf/starpdf-client"),
      import("@/lib/pdf/starpdf-page-worker-client"),
      import("jszip").then((module) => module.default),
    ]);
    const input = await bytes(files[0]);
    const handle = await StarPdfClient.open(input);
    const pageCount = await handle.getPageCount();
    await handle.close();
    const outputs = await splitStarPdfDocument(input, Array.from({ length: pageCount }, (_, page) => ({ start: page, endExclusive: page + 1 })));
    const archive = new JSZip();
    outputs.forEach((output, page) => archive.file(`page-${page + 1}.pdf`, output));
    const blob = await archive.generateAsync({ type: "blob", mimeType: "application/zip" });
    return { blob, fileName: "split-pages.zip" };
  }

  if (["pdf-delete-pages", "pdf-extract-pages", "pdf-reorder"].includes(toolId)) {
    const [{ StarPdfClient }, { mergeStarPdfDocuments }] = await Promise.all([
      import("@/lib/pdf/starpdf-client"), import("@/lib/pdf/starpdf-page-worker-client")
    ]);
    const input = await bytes(files[0]);
    const handle = await StarPdfClient.open(input);
    const pageCount = await handle.getPageCount();
    await handle.close();
    let selected: number[];
    if (toolId === "pdf-reorder") {
      selected = option.trim() ? parsePages(option, pageCount) : Array.from({ length: pageCount }, (_, index) => pageCount - index - 1);
      if (selected.length !== pageCount || new Set(selected).size !== pageCount) throw new Error("Reorder must list every page exactly once.");
    } else {
      const requested = parsePages(option || "1", pageCount);
      selected = toolId === "pdf-extract-pages" ? requested : Array.from({ length: pageCount }, (_, index) => index).filter((index) => !requested.includes(index));
      if (!selected.length) throw new Error("At least one page must remain.");
    }
    const output = await mergeStarPdfDocuments([input], selected.map((pageIndex) => ({ documentIndex: 0, pageIndex })));
    await validatePdf(output);
    return pdfOutput(output, toolId === "pdf-extract-pages" ? "extracted.pdf" : toolId === "pdf-reorder" ? "reordered.pdf" : "pages-removed.pdf");
  }

  if (toolId === "pdf-images-to-pdf") {
    const { PDFDocument } = await import("pdf-lib");
    const document = await PDFDocument.create();
    for (const file of files) {
      const data = await file.arrayBuffer();
      let image;
      if (file.type === "image/png") image = await document.embedPng(data);
      else if (file.type === "image/jpeg") image = await document.embedJpg(data);
      else {
        const bitmap = await createImageBitmap(file, { imageOrientation: "from-image" });
        const canvas = globalThis.document.createElement("canvas");
        canvas.width = bitmap.width; canvas.height = bitmap.height;
        canvas.getContext("2d")?.drawImage(bitmap, 0, 0);
        bitmap.close();
        const png = await new Promise<Blob>((resolve, reject) => canvas.toBlob((blob) => blob ? resolve(blob) : reject(new Error("Could not convert the image.")), "image/png"));
        image = await document.embedPng(await png.arrayBuffer());
      }
      const page = document.addPage([image.width, image.height]);
      page.drawImage(image, { x: 0, y: 0, width: image.width, height: image.height });
    }
    const output = await document.save();
    await validatePdf(output);
    return pdfOutput(output, "images.pdf");
  }

  const { document } = await loadPdfLib(files[0]);
  const pages = document.getPages();
  if (toolId === "pdf-rotate") {
    const { degrees } = await import("pdf-lib");
    const angle = Number(option || 90);
    if (![90, 180, 270].includes(angle)) throw new Error("Rotation must be 90, 180, or 270 degrees.");
    pages.forEach((page) => page.setRotation(degrees((page.getRotation().angle + angle) % 360)));
  } else if (toolId === "pdf-page-numbers") {
    const { StandardFonts, rgb } = await import("pdf-lib");
    const font = await document.embedFont(StandardFonts.Helvetica);
    pages.forEach((page, index) => {
      const label = `${option || "Page "}${index + 1}`;
      page.drawText(label, { x: Math.max(24, (page.getWidth() - font.widthOfTextAtSize(label, 10)) / 2), y: 18, size: 10, font, color: rgb(0.2, 0.2, 0.2) });
    });
  } else if (toolId === "pdf-watermark") {
    const { StandardFonts, degrees, rgb } = await import("pdf-lib");
    const font = await document.embedFont(StandardFonts.HelveticaBold);
    const label = option.trim() || "CONFIDENTIAL";
    pages.forEach((page) => page.drawText(label, { x: page.getWidth() * 0.2, y: page.getHeight() * 0.5, size: 42, font, rotate: degrees(35), color: rgb(0.65, 0.65, 0.65), opacity: 0.35 }));
  } else if (toolId === "pdf-metadata") {
    const [title, author, subject, keywords] = option.split("|").map((value) => value.trim());
    document.setTitle(title || files[0].name.replace(/\.pdf$/i, ""));
    if (author) document.setAuthor(author);
    if (subject) document.setSubject(subject);
    if (keywords) document.setKeywords(keywords.split(",").map((value) => value.trim()).filter(Boolean));
    document.setProducer("ToolRakyat"); document.setModificationDate(new Date());
  } else if (toolId === "pdf-sign") {
    const signatureFile = files[1];
    if (!signatureFile || !["image/png", "image/jpeg"].includes(signatureFile.type)) throw new Error("Choose a PDF followed by a PNG or JPEG signature image.");
    const signatureBytes = await signatureFile.arrayBuffer();
    const signature = signatureFile.type === "image/png" ? await document.embedPng(signatureBytes) : await document.embedJpg(signatureBytes);
    const size = signature.scale(Math.min(1, 160 / signature.width, 80 / signature.height));
    pages[pages.length - 1].drawImage(signature, { x: 48, y: 48, width: size.width, height: size.height });
  } else {
    throw new Error("Unsupported PDF operation.");
  }
  const output = await document.save();
  await validatePdf(output);
  return pdfOutput(output, `${toolId.replace("pdf-", "")}.pdf`);
}
