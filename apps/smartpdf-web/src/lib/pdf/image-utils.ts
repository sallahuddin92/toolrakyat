/**
 * Client-side utility for ensuring image files/bytes are formatted as JPEG
 * for StarPDF image insertion/stamps. 100% local-first, 0 network bytes.
 */
export async function convertToJpegBytes(
  fileOrBytes: File | Uint8Array,
): Promise<{ bytes: Uint8Array; mimeType: string }> {
  if (fileOrBytes instanceof Uint8Array) {
    // Check for JPEG magic bytes (FF D8 FF)
    if (
      fileOrBytes.length >= 3 &&
      fileOrBytes[0] === 0xff &&
      fileOrBytes[1] === 0xd8 &&
      fileOrBytes[2] === 0xff
    ) {
      return { bytes: fileOrBytes, mimeType: "image/jpeg" };
    }
  }

  // If in browser environment, convert image via canvas
  if (typeof window !== "undefined" && typeof document !== "undefined") {
    let blob: Blob;
    if (fileOrBytes instanceof File) {
      blob = fileOrBytes;
    } else {
      blob = new Blob([fileOrBytes as Uint8Array<ArrayBuffer>]);
    }

    const img = new Image();
    const url = URL.createObjectURL(blob);

    try {
      await new Promise<void>((resolve, reject) => {
        img.onload = () => resolve();
        img.onerror = (e) => reject(new Error(`Failed to load image: ${e}`));
        img.src = url;
      });

      const canvas = document.createElement("canvas");
      canvas.width = Math.max(1, img.naturalWidth || img.width || 100);
      canvas.height = Math.max(1, img.naturalHeight || img.height || 100);

      const ctx = canvas.getContext("2d");
      if (!ctx) {
        throw new Error("Canvas 2D context unavailable.");
      }

      // Fill white background for transparent PNG signatures
      ctx.fillStyle = "#ffffff";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.drawImage(img, 0, 0);

      const jpegBlob = await new Promise<Blob | null>((resolve) =>
        canvas.toBlob(resolve, "image/jpeg", 0.92),
      );

      if (!jpegBlob) {
        throw new Error("Failed to export canvas to JPEG blob.");
      }

      const buffer = await jpegBlob.arrayBuffer();
      return { bytes: new Uint8Array(buffer), mimeType: "image/jpeg" };
    } finally {
      URL.revokeObjectURL(url);
    }
  }

  // Fallback for node or tests if given raw bytes
  const bytes =
    fileOrBytes instanceof File
      ? new Uint8Array(await fileOrBytes.arrayBuffer())
      : fileOrBytes;
  return { bytes, mimeType: "image/jpeg" };
}
