import type { StarPdfDocumentHandle } from "../starpdf-client";
import type { DocumentInspectionResult, ExportMode } from "../pdf-types";
import type { SmartPdfSelection } from "../selection/types";

export type CommandStatus = "IDLE" | "RUNNING";

export type CommandExecutionState =
  | { status: "IDLE" }
  | { status: "RUNNING"; commandId: string; label: string };

export interface SmartPdfCommandContext {
  sourceBytes: Uint8Array | null;
  filename: string;
  currentPage: number; // 1-indexed
  pageCount: number;
  selection: SmartPdfSelection;
  starPdfDoc: StarPdfDocumentHandle | null;
  fieldValues: Record<string, string | boolean | string[]>;
  annotationValues: Record<string, string>;
  inspectionResult: DocumentInspectionResult | null;
  exportMode?: ExportMode;
}

export interface SmartPdfCommandResult {
  /**
   * Updated PDF binary bytes if the command produced a new document state.
   */
  bytes?: Uint8Array;
  /**
   * Target page to navigate/clamp to after execution (1-indexed).
   */
  nextPage?: number;
  /**
   * Explicit flag indicating selection should be cleared.
   */
  clearSelection?: boolean;
  /**
   * Explicit updated selection if known.
   */
  nextSelection?: SmartPdfSelection;
  /**
   * Concise success message for status bar / toast.
   */
  message?: string;
  /**
   * Updated form fields dictionary if modified.
   */
  fieldValues?: Record<string, string | boolean | string[]>;
  /**
   * Updated annotation values dictionary if modified.
   */
  annotationValues?: Record<string, string>;
  /**
   * Export download payload if this command triggers a browser download.
   */
  download?: {
    filename: string;
    bytes: Uint8Array;
  };
}

export interface SmartPdfCommand<TResult extends SmartPdfCommandResult = SmartPdfCommandResult> {
  readonly id: string;
  readonly label: string;
  readonly isMutating: boolean;
  execute(context: SmartPdfCommandContext): Promise<TResult>;
}
