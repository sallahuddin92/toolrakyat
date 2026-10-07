export type PdfPaletteCommandId =
  | "add-text" | "add-image" | "search" | "delete" | "duplicate"
  | "rotate" | "extract" | "properties" | "bookmarks"
  | "attachments" | "export";

export interface PdfPaletteCommand {
  id: PdfPaletteCommandId;
  label: string;
  keywords: string;
}

export const PDF_PALETTE_COMMANDS: readonly PdfPaletteCommand[] = [
  { id: "add-text", label: "Add text", keywords: "type insert freetext" },
  { id: "add-image", label: "Add image", keywords: "picture photo insert" },
  { id: "search", label: "Search", keywords: "find text" },
  { id: "delete", label: "Delete", keywords: "remove selection page" },
  { id: "duplicate", label: "Duplicate", keywords: "copy page" },
  { id: "rotate", label: "Rotate view", keywords: "turn page clockwise" },
  { id: "extract", label: "Extract pages", keywords: "separate selected pages" },
  { id: "properties", label: "Properties", keywords: "document info details" },
  { id: "bookmarks", label: "Bookmarks", keywords: "outline navigation" },
  { id: "attachments", label: "Attachments", keywords: "embedded files" },
  { id: "export", label: "Export", keywords: "download save pdf" },
];

export function searchPdfPalette(query: string): PdfPaletteCommand[] {
  const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  if (!terms.length) return [...PDF_PALETTE_COMMANDS];
  return PDF_PALETTE_COMMANDS.map((command, index) => {
    const label = command.label.toLocaleLowerCase();
    const haystack = `${label} ${command.keywords}`;
    if (!terms.every((term) => haystack.includes(term))) return null;
    const score = terms.reduce((total, term) => total +
      (label === term ? 0 : label.startsWith(term) ? 1 : label.includes(term) ? 2 : 3), 0);
    return { command, index, score };
  }).filter((item): item is { command: PdfPaletteCommand; index: number; score: number } => item !== null)
    .sort((a, b) => a.score - b.score || a.index - b.index)
    .map((item) => item.command);
}
