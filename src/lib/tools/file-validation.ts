import { fileTypeFromBuffer } from "file-type";
import { LIMITS } from "../limits";

export interface FileValidationOptions {
  file: File;
  allowedTypes: string[];
  maxSizeMB: number;
}

export type FileValidationResult =
  | { ok: true; detectedMime?: string }
  | { ok: false; error: string };

function getExtension(filename: string) {
  const idx = filename.lastIndexOf(".");
  if (idx < 0) return "";
  return filename.slice(idx + 1).toLowerCase();
}

// Keep this conservative; expand as tools require.
const EXT_BY_MIME: Record<string, string[]> = {
  "application/pdf": ["pdf"],
  "application/zip": ["zip"],
  "image/jpeg": ["jpg", "jpeg"],
  "image/png": ["png"],
  "image/webp": ["webp"],
};

export async function validateUploadedFile(
  options: FileValidationOptions,
): Promise<FileValidationResult> {
  const { file, allowedTypes, maxSizeMB } = options;

  if (!file) return { ok: false, error: "No file selected." };

  // Enforce global hard limit first
  const globalMaxMB = LIMITS.GLOBAL_MAX_FILE_SIZE_MB;
  if (file.size > globalMaxMB * 1024 * 1024) {
    return {
      ok: false,
      error: `File is too large. Maximum allowed size is ${globalMaxMB}MB for production safety.`,
    };
  }

  // Enforce per-tool limit
  if (file.size > maxSizeMB * 1024 * 1024) {
    return { ok: false, error: `File too large. Max size is ${maxSizeMB} MB.` };
  }

  // Extension check (best-effort); MIME sniffing is the real check.
  const ext = getExtension(file.name || "");
  const allowedExts = new Set(
    allowedTypes.flatMap((m: string) => EXT_BY_MIME[m] ?? []),
  );
  if (allowedExts.size > 0 && ext && !allowedExts.has(ext)) {
    return { ok: false, error: "File extension not supported for this tool." };
  }

  // MIME sniffing: read the entire file buffer for now. If this becomes a
  // memory issue, we can switch to partial reads + streaming workflows.
  const buf = Buffer.from(await file.arrayBuffer());
  const detected = await fileTypeFromBuffer(buf);
  const detectedMime = detected?.mime;

  if (detectedMime && !allowedTypes.includes(detectedMime)) {
    return { ok: false, error: "File type not supported." };
  }

  // Fallback: if file-type can't detect, fall back to browser-provided type.
  if (!detectedMime && file.type && !allowedTypes.includes(file.type)) {
    return { ok: false, error: "File type not supported." };
  }

  return { ok: true, detectedMime };
}

