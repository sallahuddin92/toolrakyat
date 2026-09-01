import type { UtilityOutput } from "./pdf-utilities";

function loadImage(file: File): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const image = new Image();
    image.onload = () => { URL.revokeObjectURL(url); resolve(image); };
    image.onerror = () => { URL.revokeObjectURL(url); reject(new Error("The selected image could not be decoded.")); };
    image.src = url;
  });
}

function canvasBlob(canvas: HTMLCanvasElement, type: string, quality?: number): Promise<Blob> {
  return new Promise((resolve, reject) => canvas.toBlob(
    (blob) => blob ? resolve(blob) : reject(new Error("The browser could not encode the output image.")),
    type,
    quality
  ));
}

export async function processImageUtility(toolId: string, file: File, option: string): Promise<UtilityOutput> {
  const image = await loadImage(file);
  let width = image.naturalWidth;
  let height = image.naturalHeight;
  let rotate = 0;
  let flipX = false;
  let flipY = false;
  let type = file.type === "image/png" ? "image/png" : "image/jpeg";
  let quality = 0.82;

  if (toolId === "image-resize") {
    const [requestedWidth, requestedHeight] = option.split(/[x,\s]+/).map(Number);
    if (!Number.isFinite(requestedWidth) || requestedWidth < 1 || requestedWidth > 12_000) throw new Error("Enter dimensions such as 1200x800.");
    width = Math.round(requestedWidth);
    height = Number.isFinite(requestedHeight) && requestedHeight > 0 ? Math.round(requestedHeight) : Math.round(image.naturalHeight * width / image.naturalWidth);
  } else if (toolId === "image-convert") {
    type = option === "png" ? "image/png" : option === "webp" ? "image/webp" : "image/jpeg";
  } else if (toolId === "image-rotate") {
    rotate = Number(option || 90);
    if (![90, 180, 270].includes(rotate)) throw new Error("Rotation must be 90, 180, or 270 degrees.");
  } else if (toolId === "image-flip") {
    flipY = option === "vertical";
    flipX = !flipY;
  } else if (toolId === "image-compress") {
    quality = Math.min(1, Math.max(0.1, Number(option || 75) / 100));
    if (file.type !== "image/png") type = "image/jpeg";
  } else if (toolId === "image-remove-metadata") {
    type = file.type === "image/png" ? "image/png" : "image/jpeg";
    quality = 0.94;
  }

  const swap = rotate === 90 || rotate === 270;
  const canvas = document.createElement("canvas");
  canvas.width = swap ? height : width;
  canvas.height = swap ? width : height;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("Canvas processing is unavailable in this browser.");
  if (type === "image/jpeg") { context.fillStyle = "#ffffff"; context.fillRect(0, 0, canvas.width, canvas.height); }
  context.translate(canvas.width / 2, canvas.height / 2);
  context.rotate(rotate * Math.PI / 180);
  context.scale(flipX ? -1 : 1, flipY ? -1 : 1);
  context.drawImage(image, -width / 2, -height / 2, width, height);
  const blob = await canvasBlob(canvas, type, quality);
  if (!blob.size || !blob.type.startsWith("image/")) throw new Error("Image encoding produced an invalid output.");
  const extension = type === "image/png" ? "png" : type === "image/webp" ? "webp" : "jpg";
  return { blob, fileName: `${file.name.replace(/\.[^.]+$/, "")}-${toolId.replace("image-", "")}.${extension}` };
}
