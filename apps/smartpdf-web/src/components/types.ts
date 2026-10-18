import type { EditorMode } from "./workspace/PdfToolbar";

export type SmartPdfSurface = "home" | "tools" | "workspace";

export type PdfWorkflowCategory =
  | "edit"
  | "pages"
  | "forms"
  | "annotate"
  | "security";

export interface PdfToolDefinition {
  id: string;
  title: string;
  description: string;
  category: PdfWorkflowCategory;
  categoryLabel: string;
  iconName: string;
  recommendedMode?: EditorMode;
  shortcut?: string;
  actionType: "open_mode" | "open_dialog" | "custom";
  dialogTarget?: "merge" | "split" | "info";
}

export interface RecentDocumentMeta {
  id: string;
  name: string;
  sizeBytes: number;
  lastOpened: number;
  pageCount?: number;
}

export interface DocumentTabSession {
  id: string;
  title: string;
  sourceBytes: Uint8Array;
  isModified: boolean;
  pageCount: number;
  initialMode?: EditorMode;
}
