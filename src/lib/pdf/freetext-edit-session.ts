import type { PdfMarkupAnnotation } from "./pdf-types";
import type { PdfRect } from "./selection";
import type { StarPdfUpdateAnnotationInput } from "./starpdf-types";
import {
  pushHistorySnapshot,
  type SmartPdfHistoryState,
} from "./commands/history";

export const FREETEXT_EDIT_DEBOUNCE_MS = 220;

export interface FreeTextEditState {
  contents: string;
  fontFamily: "SansSerif" | "Serif" | "Monospace";
  fontSize: number;
  bold: boolean;
  italic: boolean;
  underline: boolean;
  strikethrough: boolean;
  textColor: [number, number, number];
  highlightEnabled: boolean;
  highlightColor: [number, number, number];
  rect: PdfRect;
}

export interface FreeTextEditSession {
  id: number;
  annotationId: string;
  pageIndex: number;
  objectNumber: number;
  generationNumber: number;
  originalState: FreeTextEditState;
  draftState: FreeTextEditState;
  lastAppliedState: FreeTextEditState;
  originalBytes: Uint8Array;
  latestBytes: Uint8Array;
  originalIsModified: boolean;
  revision: number;
  appliedRevision: number;
  refreshedRevision: number;
  dirty: boolean;
  syncing: boolean;
}

export function commitFreeTextSessionHistory(
  history: SmartPdfHistoryState,
  session: FreeTextEditSession,
): SmartPdfHistoryState {
  if (!session.dirty) return history;
  return pushHistorySnapshot(history, session.latestBytes, "Edit FreeText");
}

export function getFreeTextSessionCancelState(session: FreeTextEditSession): {
  bytes: Uint8Array;
  isModified: boolean;
} {
  return {
    bytes: session.originalBytes.slice(0),
    isModified: session.originalIsModified,
  };
}

export function freeTextSessionNeedsDocumentRestore(
  session: FreeTextEditSession,
): boolean {
  // A revision means updateAnnotation may already have changed the live
  // StarPDF registry even when export or the subsequent UI refresh failed.
  return session.revision > 0;
}

export function recordFreeTextExportedRevision(
  session: FreeTextEditSession,
  revision: number,
  state: FreeTextEditState,
  bytes: Uint8Array,
): void {
  if (revision < session.appliedRevision) return;
  session.lastAppliedState = cloneFreeTextState(state);
  session.latestBytes = bytes;
  session.appliedRevision = revision;
}

export function freeTextSessionNeedsSync(session: FreeTextEditSession): boolean {
  return (
    session.appliedRevision < session.revision ||
    session.refreshedRevision < session.revision
  );
}

export async function refreshFreeTextExportedRevision<T>(
  session: FreeTextEditSession,
  revision: number,
  refresh: () => Promise<T>,
): Promise<T> {
  const result = await refresh();
  if (session.revision === revision && session.appliedRevision >= revision) {
    session.refreshedRevision = revision;
  }
  return result;
}

/**
 * Dependency seam for {@link drainFreeTextSyncQueue}.
 *
 * The editor passes live refs and its queued revision applier; tests pass a
 * plain session plus an injectable applier so that refresh-failure recovery can
 * be exercised without a DOM.
 */
export interface FreeTextSyncQueueDeps {
  /** Current session, or null when the session was cancelled/closed. */
  getSession: () => FreeTextEditSession | null;
  /** Resolves once any already-queued revision application has settled. */
  awaitPendingApply: () => Promise<unknown>;
  /**
   * Applies one revision: native annotation update, incremental export, and the
   * authoritative UI refresh. May reject (e.g. refresh failure after the bytes
   * were already produced).
   */
  applyRevision: (
    sessionId: number,
    revision: number,
    draft: FreeTextEditState,
  ) => Promise<void>;
}

/**
 * Drains the FreeText synchronization queue until the session is fully synced.
 *
 * This is the exact loop the editor runs before export/undo/redo/cancel. It is
 * extracted here so the failure and progress semantics are testable with an
 * injected refresh failure:
 *
 *  - if `applyRevision` rejects, the error propagates to the caller and the
 *    session is left unsynchronized (`freeTextSessionNeedsSync` stays true), so
 *    the next flush must retry rather than silently accept stale state;
 *  - if a revision makes no observable progress, it throws
 *    "FreeText revision did not make progress." instead of spinning forever.
 */
export async function drainFreeTextSyncQueue(
  deps: FreeTextSyncQueueDeps,
): Promise<void> {
  await deps.awaitPendingApply();

  while (true) {
    const session = deps.getSession();
    if (!session || !freeTextSessionNeedsSync(session)) return;
    const revision = session.revision;
    const draft = cloneFreeTextState(session.draftState);
    const previousApplied = session.appliedRevision;
    const previousRefreshed = session.refreshedRevision;
    await deps.applyRevision(session.id, revision, draft);
    const latest = deps.getSession();
    if (!latest || latest.id !== session.id || !freeTextSessionNeedsSync(latest)) return;
    if (
      latest.revision === revision &&
      latest.appliedRevision === previousApplied &&
      latest.refreshedRevision === previousRefreshed
    ) {
      throw new Error("FreeText revision did not make progress.");
    }
  }
}

const sameColor = (
  left: [number, number, number],
  right: [number, number, number],
) => left.every((channel, index) => Math.round(channel * 255) === Math.round(right[index] * 255));

export function freeTextStateFromAnnotation(annotation: PdfMarkupAnnotation): FreeTextEditState {
  return {
    contents: annotation.contents ?? "",
    fontFamily: annotation.fontFamily ?? "SansSerif",
    fontSize: annotation.fontSize ?? 12,
    bold: Boolean(annotation.isBold),
    italic: Boolean(annotation.isItalic),
    underline: Boolean(annotation.isUnderlined),
    strikethrough: Boolean(annotation.isStruckThrough),
    textColor: annotation.textColor ? [...annotation.textColor] : [0, 0, 0],
    highlightEnabled: Boolean(annotation.highlightColor),
    highlightColor: annotation.highlightColor ? [...annotation.highlightColor] : [1, 0.92, 0.23],
    rect: { ...annotation.rect },
  };
}

export function cloneFreeTextState(state: FreeTextEditState): FreeTextEditState {
  return {
    ...state,
    textColor: [...state.textColor],
    highlightColor: [...state.highlightColor],
    rect: { ...state.rect },
  };
}

export function freeTextStatesEqual(
  left: FreeTextEditState,
  right: FreeTextEditState,
): boolean {
  return (
    left.contents === right.contents &&
    left.fontFamily === right.fontFamily &&
    Math.abs(left.fontSize - right.fontSize) < 0.001 &&
    left.bold === right.bold &&
    left.italic === right.italic &&
    left.underline === right.underline &&
    left.strikethrough === right.strikethrough &&
    sameColor(left.textColor, right.textColor) &&
    left.highlightEnabled === right.highlightEnabled &&
    sameColor(left.highlightColor, right.highlightColor) &&
    Math.abs(left.rect.x - right.rect.x) < 0.001 &&
    Math.abs(left.rect.y - right.rect.y) < 0.001 &&
    Math.abs(left.rect.width - right.rect.width) < 0.001 &&
    Math.abs(left.rect.height - right.rect.height) < 0.001
  );
}

export function freeTextStateToUpdate(state: FreeTextEditState): StarPdfUpdateAnnotationInput {
  return {
    contents: state.contents,
    font_family: state.fontFamily,
    font_size: state.fontSize,
    bold: state.bold,
    italic: state.italic,
    text_color: [...state.textColor],
    underline: state.underline,
    strikethrough: state.strikethrough,
    highlight_enabled: state.highlightEnabled,
    ...(state.highlightEnabled ? { highlight_color: [...state.highlightColor] } : {}),
    rect: [
      state.rect.x,
      state.rect.y,
      state.rect.x + state.rect.width,
      state.rect.y + state.rect.height,
    ],
  };
}

export function freeTextStateToUpdatePatch(
  previous: FreeTextEditState,
  next: FreeTextEditState,
): StarPdfUpdateAnnotationInput {
  const update: StarPdfUpdateAnnotationInput = {};
  if (previous.contents !== next.contents) update.contents = next.contents;
  if (previous.fontFamily !== next.fontFamily) update.font_family = next.fontFamily;
  if (Math.abs(previous.fontSize - next.fontSize) >= 0.001) update.font_size = next.fontSize;
  if (previous.bold !== next.bold) update.bold = next.bold;
  if (previous.italic !== next.italic) update.italic = next.italic;
  if (!sameColor(previous.textColor, next.textColor)) update.text_color = [...next.textColor];
  if (previous.underline !== next.underline) update.underline = next.underline;
  if (previous.strikethrough !== next.strikethrough) {
    update.strikethrough = next.strikethrough;
  }
  if (previous.highlightEnabled !== next.highlightEnabled) {
    update.highlight_enabled = next.highlightEnabled;
  }
  if (
    next.highlightEnabled &&
    (previous.highlightEnabled !== next.highlightEnabled ||
      !sameColor(previous.highlightColor, next.highlightColor))
  ) {
    update.highlight_color = [...next.highlightColor];
  }
  if (
    Math.abs(previous.rect.x - next.rect.x) >= 0.001 ||
    Math.abs(previous.rect.y - next.rect.y) >= 0.001 ||
    Math.abs(previous.rect.width - next.rect.width) >= 0.001 ||
    Math.abs(previous.rect.height - next.rect.height) >= 0.001
  ) {
    update.rect = [
      next.rect.x,
      next.rect.y,
      next.rect.x + next.rect.width,
      next.rect.y + next.rect.height,
    ];
  }
  return update;
}

export function annotationWithFreeTextState(
  annotation: PdfMarkupAnnotation,
  state: FreeTextEditState,
): PdfMarkupAnnotation {
  return {
    ...annotation,
    contents: state.contents,
    rect: { ...state.rect },
    fontFamily: state.fontFamily,
    fontSize: state.fontSize,
    isBold: state.bold,
    isItalic: state.italic,
    isUnderlined: state.underline,
    isStruckThrough: state.strikethrough,
    textColor: [...state.textColor],
    highlightColor: state.highlightEnabled ? [...state.highlightColor] : undefined,
  };
}
