"use client";

import { useEffect, useMemo, useState } from "react";
import { Check, Copy, Download, Loader2, RotateCcw } from "lucide-react";
import toast from "react-hot-toast";

import { ResultDownloadCard } from "@/components/tools/ResultDownloadCard";
import { ToolError } from "@/components/tools/ToolError";
import { ToolSettingsPanel } from "@/components/tools/ToolSettingsPanel";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import type { ToolDefinition } from "@/lib/tools/types";
import { generatePassword, processTextUtility } from "@/lib/tools/utility-processors";

type DownloadResult = { blob: Blob; fileName: string; preview?: string };

const FILE_TOOLS = new Set([
  "pdf-merge", "pdf-split", "pdf-rotate", "pdf-images-to-pdf", "pdf-delete-pages",
  "pdf-extract-pages", "pdf-page-numbers", "pdf-watermark", "pdf-metadata", "pdf-sign", "pdf-reorder",
  "image-compress", "image-resize", "image-convert", "image-rotate", "image-flip", "image-remove-metadata",
  "compression-batch-image-compress", "zip-create", "zip-extract",
]);

const OPTION_LABELS: Record<string, string> = {
  "pdf-delete-pages": "Pages to delete (for example 1-3,5)", "pdf-extract-pages": "Pages to extract",
  "pdf-reorder": "New page order (leave blank to reverse)", "pdf-rotate": "Rotation (90, 180, or 270)",
  "pdf-page-numbers": "Page number prefix", "pdf-watermark": "Watermark text",
  "pdf-metadata": "Title | author | subject | keywords", "image-resize": "Dimensions (for example 1200x800)",
  "image-convert": "Output format (jpg, png, or webp)", "image-rotate": "Rotation (90, 180, or 270)",
  "image-flip": "Direction (horizontal or vertical)", "image-compress": "Quality (10-100)",
  "text-case-converter": "Mode (upper, lower, title, or sentence)", "dev-base64": "Mode (encode or decode)",
  "dev-url-encode": "Mode (encode or decode)",
};

const DEFAULT_OPTIONS: Record<string, string> = {
  "pdf-delete-pages": "1", "pdf-extract-pages": "1", "pdf-rotate": "90", "pdf-page-numbers": "Page ",
  "pdf-watermark": "CONFIDENTIAL", "image-resize": "1200", "image-convert": "jpg",
  "image-rotate": "90", "image-flip": "horizontal", "image-compress": "75", "text-case-converter": "upper",
  "dev-base64": "encode", "dev-url-encode": "encode",
};

const TEXT_PLACEHOLDERS: Record<string, string> = {
  "converter-csv-to-json": "name,amount\nAli,25.50", "converter-json-to-csv": '[{"name":"Ali","amount":25.5}]',
  "compression-json-minifier": '{\n  "hello": "world"\n}', "dev-json-formatter": '{"hello":"world"}',
  "dev-json-validator": '{"valid":true}', "calc-loan": "100000, 4.5, 30", "calc-profit-margin": "60, 100",
  "calc-discount": "100, 20", "calc-sst": "100, 8", "calc-bmi": "70, 175",
  "calc-compound-interest": "10000, 5, 10, 12", "calc-age": "1990-01-01",
  "calc-date-difference": "2026-01-01 2026-02-01", "dev-color-converter": "#0EA5E9",
};

function accepts(tool: ToolDefinition): string | undefined {
  return tool.acceptedFileTypes?.join(",");
}

async function sha256(input: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(input));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function businessDocument(tool: ToolDefinition, input: string): DownloadResult {
  const escaped = input.replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[char]!);
  const html = `<!doctype html><html><head><meta charset="utf-8"><title>${tool.name}</title><style>body{font:16px system-ui;max-width:760px;margin:48px auto;color:#0f172a}h1{border-bottom:2px solid #0ea5e9;padding-bottom:12px}pre{white-space:pre-wrap;line-height:1.6}</style></head><body><h1>${tool.name}</h1><pre>${escaped}</pre><script>window.print()<\/script></body></html>`;
  return { blob: new Blob([html], { type: "text/html" }), fileName: `${tool.slug}.html`, preview: "Printable document generated." };
}

export function GenericUtilityTool({ tool }: { tool: ToolDefinition }) {
  const fileMode = FILE_TOOLS.has(tool.id);
  const [input, setInput] = useState("");
  const [option, setOption] = useState(DEFAULT_OPTIONS[tool.id] ?? "");
  const [files, setFiles] = useState<File[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [textResult, setTextResult] = useState("");
  const [download, setDownload] = useState<(DownloadResult & { url: string }) | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => () => { if (download) URL.revokeObjectURL(download.url); }, [download]);
  const multiple = useMemo(() => ["pdf-merge", "pdf-images-to-pdf", "pdf-sign", "zip-create", "compression-batch-image-compress"].includes(tool.id), [tool.id]);

  const reset = () => {
    if (download) URL.revokeObjectURL(download.url);
    setInput(""); setFiles([]); setTextResult(""); setDownload(null); setError(""); setCopied(false);
  };

  const publish = (result: DownloadResult) => {
    if (!result.blob.size) throw new Error("Processing produced an empty output.");
    if (download) URL.revokeObjectURL(download.url);
    setDownload({ ...result, url: URL.createObjectURL(result.blob) });
    setTextResult(result.preview ?? "");
  };

  const process = async () => {
    setBusy(true); setError(""); setTextResult("");
    try {
      if (fileMode) {
        if (!files.length) throw new Error("Choose at least one file.");
        for (const file of files) {
          if (tool.maxFileSizeMB && file.size > tool.maxFileSizeMB * 1024 * 1024) throw new Error(`${file.name} exceeds the ${tool.maxFileSizeMB} MB limit.`);
          const accepted = tool.acceptedFileTypes ?? [];
          if (accepted.length && file.type && !accepted.includes(file.type)) throw new Error(`${file.name} has an unsupported file type.`);
        }
        if (tool.categoryId === "pdf") {
          publish(await (await import("@/lib/tools/pdf-utilities")).processPdfUtility(tool.id, files, option));
        } else if (tool.categoryId === "image") {
          publish(await (await import("@/lib/tools/image-utilities")).processImageUtility(tool.id, files[0], option));
        } else {
          const JSZip = (await import("jszip")).default;
          if (tool.id === "zip-create") {
            const zip = new JSZip(); files.forEach((file) => zip.file(file.name, file));
            publish({ blob: await zip.generateAsync({ type: "blob", mimeType: "application/zip" }), fileName: "archive.zip" });
          } else if (tool.id === "zip-extract") {
            const zip = await JSZip.loadAsync(files[0]);
            const output = new JSZip();
            const entries = Object.values(zip.files).filter((entry) => !entry.dir);
            if (!entries.length) throw new Error("The archive contains no files.");
            for (const entry of entries) output.file(entry.name, await entry.async("uint8array"));
            publish({ blob: await output.generateAsync({ type: "blob", mimeType: "application/zip" }), fileName: "extracted-files.zip", preview: `${entries.length} archive entries validated.` });
          } else {
            const output = new JSZip();
            for (const file of files) {
              const result = await (await import("@/lib/tools/image-utilities")).processImageUtility("image-compress", file, option || "75");
              output.file(result.fileName, result.blob);
            }
            publish({ blob: await output.generateAsync({ type: "blob", mimeType: "application/zip" }), fileName: "compressed-images.zip" });
          }
        }
      } else if (tool.categoryId === "business") {
        if (!input.trim()) throw new Error("Enter document details first.");
        publish(businessDocument(tool, input));
      } else if (tool.id === "text-password-generator") {
        setTextResult(generatePassword(Number(input || 20)));
      } else if (tool.id === "dev-uuid-generator") {
        const count = Math.min(100, Math.max(1, Number(input || 1)));
        setTextResult(Array.from({ length: count }, () => crypto.randomUUID()).join("\n"));
      } else if (tool.id === "dev-sha-256") {
        setTextResult(await sha256(input));
      } else if (tool.id === "converter-markdown-to-html") {
        const { marked } = await import("marked");
        const output = await marked.parse(input, { async: false });
        publish({ blob: new Blob([output], { type: "text/html" }), fileName: "converted.html", preview: output });
      } else if (tool.categoryId === "qr") {
        const QRCode = (await import("qrcode")).default;
        let value = input;
        if (tool.id === "qr-whatsapp-link-generator") value = `https://wa.me/${input.replace(/\D/g, "")}`;
        if (tool.id === "qr-wifi") value = `WIFI:T:WPA;S:${input};P:${option};;`;
        if (tool.id === "qr-vcard") value = `BEGIN:VCARD\nVERSION:3.0\nFN:${input}\nEND:VCARD`;
        if (tool.id === "qr-email") value = `mailto:${input}`;
        if (tool.id === "qr-sms") value = `sms:${input}`;
        if (!value.trim()) throw new Error("Enter content to encode.");
        const dataUrl = await QRCode.toDataURL(value, { errorCorrectionLevel: "M", width: 512 });
        const blob = await fetch(dataUrl).then((response) => response.blob());
        publish({ blob, fileName: `${tool.slug}.png`, preview: value });
      } else {
        const result = processTextUtility(tool.id, input, option);
        setTextResult(result.output);
        if (result.extension) publish({ blob: new Blob([result.output], { type: result.mime }), fileName: `output.${result.extension}`, preview: result.output });
      }
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Processing failed.");
    } finally { setBusy(false); }
  };

  return (
    <div className="space-y-5" data-testid="utility-tool" data-tool-id={tool.id}>
      <Card className="rounded-2xl"><CardContent className="space-y-4 p-5 sm:p-6">
        {fileMode ? <label className="block text-sm font-medium text-slate-700">Choose {multiple ? "files" : "a file"}<Input data-testid="tool-file-input" className="mt-2" type="file" accept={accepts(tool)} multiple={multiple} onChange={(event) => setFiles(Array.from(event.target.files ?? []))} /></label> :
          <label className="block text-sm font-medium text-slate-700">Input<Textarea data-testid="tool-text-input" className="mt-2 min-h-40" maxLength={1_000_000} value={input} placeholder={TEXT_PLACEHOLDERS[tool.id] ?? "Enter or paste your content here"} onChange={(event) => setInput(event.target.value)} /></label>}
        {OPTION_LABELS[tool.id] ? <ToolSettingsPanel><label className="block text-sm font-medium text-slate-700">{OPTION_LABELS[tool.id]}<Input data-testid="tool-option-input" className="mt-2" value={option} onChange={(event) => setOption(event.target.value)} /></label></ToolSettingsPanel> : null}
        <Button data-testid="tool-primary-action" type="button" onClick={process} disabled={busy} className="rounded-xl">{busy ? <Loader2 className="mr-2 size-4 animate-spin" /> : null}{busy ? "Processing…" : "Process"}</Button>
      </CardContent></Card>
      {error ? <ToolError message={error} /> : null}
      {download ? <ResultDownloadCard fileName={download.fileName} url={download.url} bytes={download.blob.size} originalBytes={files.reduce((sum, file) => sum + file.size, 0) || undefined} onReset={reset}>{download.preview ? <pre className="max-h-48 overflow-auto whitespace-pre-wrap" data-testid="tool-result">{download.preview}</pre> : null}</ResultDownloadCard> : null}
      {textResult && !download ? <Card className="rounded-2xl"><CardContent className="space-y-3 p-5"><pre className="max-h-96 overflow-auto whitespace-pre-wrap break-words" data-testid="tool-result">{textResult}</pre><div className="flex gap-2"><Button type="button" variant="outline" onClick={async () => { await navigator.clipboard.writeText(textResult); setCopied(true); toast.success("Copied"); }}>{copied ? <Check className="mr-2 size-4" /> : <Copy className="mr-2 size-4" />}{copied ? "Copied" : "Copy"}</Button><Button type="button" variant="ghost" onClick={reset}><RotateCcw className="mr-2 size-4" />Reset</Button><Button type="button" variant="secondary" asChild><a download={`${tool.slug}.txt`} href={`data:text/plain;charset=utf-8,${encodeURIComponent(textResult)}`}><Download className="mr-2 size-4" />Download</a></Button></div></CardContent></Card> : null}
    </div>
  );
}
