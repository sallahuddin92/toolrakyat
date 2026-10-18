export interface FileValidationOptions {
  file: File;
  allowedTypes: string[];
  maxSizeMB: number;
}

export type FileValidationResult =
  | { ok: true; detectedMime?: string }
  | { ok: false; error: string };

export async function validateUploadedFile(
  options: FileValidationOptions,
): Promise<FileValidationResult> {
  const { file, allowedTypes, maxSizeMB } = options;

  if (!file) return { ok: false, error: "No file selected." };

  if (file.size > maxSizeMB * 1024 * 1024) {
    return { ok: false, error: `File too large. Max size is ${maxSizeMB} MB.` };
  }

  // Validate PDF extension and MIME
  const isPdf = file.name.toLowerCase().endsWith(".pdf") || file.type === "application/pdf";
  if (!isPdf && !allowedTypes.includes(file.type)) {
    return { ok: false, error: "File type not supported. Please select a valid PDF document." };
  }

  return { ok: true, detectedMime: file.type || "application/pdf" };
}
