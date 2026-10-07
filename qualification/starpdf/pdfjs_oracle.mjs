import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
const [pdfjsPath, pdfPath] = process.argv.slice(2);
const pdfjs = await import(pathToFileURL(pdfjsPath).href);
const data = new Uint8Array(readFileSync(pdfPath));
const task = pdfjs.getDocument({ data, useSystemFonts: true, disableFontFace: true });
const doc = await task.promise;
const pages = [];
for (let i = 1; i <= doc.numPages; i++) {
  const page = await doc.getPage(i);
  const content = await page.getTextContent();
  pages.push(content.items.map(item => item.str ?? '').join(' '));
  page.cleanup();
}
console.log(JSON.stringify({ page_count: doc.numPages, text: pages }));
await task.destroy();
