import { ContractError } from "./refusal";
import type { CoordinateConfig, CoordinateTransform, PdfBox, Point } from "./contracts";
function validBox(rect: PdfBox): boolean {
  return rect.every(Number.isFinite) && rect[0] <= rect[2] && rect[1] <= rect[3];
}
function invalid(): never { throw new ContractError("InvalidCoordinates", "invalid box, point or scale"); }
function finite(point: Point): Point { if (!point.every(Number.isFinite)) return invalid(); return point; }
export function createCoordinateTransform(config: CoordinateConfig): CoordinateTransform {
  const { media_box: media, crop_box: crop = media, rotation, scale, user_unit: unit, device_pixel_ratio: dpr } = config;
  const view: PdfBox = [Math.max(media[0], crop[0]), Math.max(media[1], crop[1]), Math.min(media[2], crop[2]), Math.min(media[3], crop[3])];
  const factor = scale * unit;
  if (!validBox(media) || !validBox(crop) || !validBox(view) || view[0] === view[2] || view[1] === view[3] ||
      ![0, 90, 180, 270].includes(rotation) || [scale, unit, dpr, factor, factor * dpr].some((v) => !Number.isFinite(v) || v <= 0)) return invalid();
  const [left, bottom, right, top] = view;
  const pdfToViewport = (point: Point): Point => {
    const [x, y] = finite(point);
    let result: Point;
    switch (rotation) {
      case 0: result = [x - left, top - y]; break;
      case 90: result = [y - bottom, x - left]; break;
      case 180: result = [right - x, y - bottom]; break;
      case 270: result = [top - y, right - x]; break;
    }
    return finite([result[0] * factor, result[1] * factor]);
  };
  return {
    pdfToViewport,
    viewportToPdf(point) {
      finite(point);
      const x = point[0] / factor, y = point[1] / factor;
      switch (rotation) {
        case 0: return finite([left + x, top - y]);
        case 90: return finite([left + y, bottom + x]);
        case 180: return finite([right - x, bottom + y]);
        case 270: return finite([right - y, top - x]);
      }
    },
    viewportToDevice(point) { finite(point); return finite([point[0] * dpr, point[1] * dpr]); },
    deviceToViewport(point) { finite(point); return finite([point[0] / dpr, point[1] / dpr]); },
    pdfBoxToViewport(rect) {
      if (!validBox(rect)) return invalid();
      const points = [[rect[0], rect[1]], [rect[0], rect[3]], [rect[2], rect[1]], [rect[2], rect[3]]] as const;
      const transformed = points.map(pdfToViewport);
      return [Math.min(...transformed.map((p) => p[0])), Math.min(...transformed.map((p) => p[1])),
              Math.max(...transformed.map((p) => p[0])), Math.max(...transformed.map((p) => p[1]))];
    },
  };
}
