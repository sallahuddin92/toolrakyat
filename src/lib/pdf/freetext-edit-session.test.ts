import { describe, expect, it } from "vitest";

import type { PdfMarkupAnnotation } from "./pdf-types";
import {
  annotationWithFreeTextState,
  cloneFreeTextState,
  commitFreeTextSessionHistory,
  freeTextStateFromAnnotation,
  freeTextSessionNeedsDocumentRestore,
  freeTextSessionNeedsSync,
  freeTextStatesEqual,
  freeTextStateToUpdate,
  freeTextStateToUpdatePatch,
  getFreeTextSessionCancelState,
  recordFreeTextExportedRevision,
  refreshFreeTextExportedRevision,
  drainFreeTextSyncQueue,
  type FreeTextEditSession,
  type FreeTextSyncQueueDeps,
} from "./freetext-edit-session";
import {
  createInitialHistoryState,
  redoHistory,
  undoHistory,
} from "./commands/history";

const annotation: PdfMarkupAnnotation = {
  id: "annot-obj-9-0",
  subtype: "FreeText",
  contents: "nama",
  rect: { x: 10, y: 20, width: 80, height: 24 },
  pageIndex: 0,
  objectNumber: 9,
  generationNumber: 0,
  fontFamily: "Serif",
  fontSize: 18,
  isBold: true,
  isItalic: false,
  textColor: [0.1, 0.2, 0.3],
  isUnderlined: true,
  isStruckThrough: false,
  highlightColor: [1, 0.8, 0.2],
};

function createSession(): FreeTextEditSession {
  const originalState = freeTextStateFromAnnotation(annotation);
  const draftState = cloneFreeTextState(originalState);
  draftState.contents = "final value after several drafts";
  return {
    id: 1,
    annotationId: annotation.id,
    pageIndex: annotation.pageIndex,
    objectNumber: annotation.objectNumber!,
    generationNumber: annotation.generationNumber!,
    originalState,
    draftState,
    lastAppliedState: cloneFreeTextState(draftState),
    originalBytes: new Uint8Array([1, 2, 3]),
    latestBytes: new Uint8Array([4, 5, 6]),
    originalIsModified: false,
    revision: 3,
    appliedRevision: 3,
    refreshedRevision: 3,
    dirty: true,
    syncing: false,
  };
}

describe("FreeText edit session state", () => {
  it("captures and clones every authoritative editable property", () => {
    const state = freeTextStateFromAnnotation(annotation);
    const clone = cloneFreeTextState(state);

    expect(clone).toEqual(state);
    expect(freeTextStatesEqual(clone, state)).toBe(true);
    clone.textColor[0] = 0.9;
    clone.rect.x = 99;
    expect(state.textColor[0]).toBe(0.1);
    expect(state.rect.x).toBe(10);
    expect(freeTextStatesEqual(clone, state)).toBe(false);
  });

  it("serializes a complete authoritative StarPDF update", () => {
    const state = freeTextStateFromAnnotation(annotation);
    expect(freeTextStateToUpdate(state)).toEqual({
      contents: "nama",
      font_family: "Serif",
      font_size: 18,
      bold: true,
      italic: false,
      text_color: [0.1, 0.2, 0.3],
      underline: true,
      strikethrough: false,
      highlight_enabled: true,
      highlight_color: [1, 0.8, 0.2],
      rect: [10, 20, 90, 44],
    });
  });

  it("serializes only fields changed since the last authoritative state", () => {
    const previous = freeTextStateFromAnnotation(annotation);
    const next = cloneFreeTextState(previous);
    next.contents = "updated";
    next.fontSize = 20;
    next.highlightEnabled = false;

    expect(freeTextStateToUpdatePatch(previous, next)).toEqual({
      contents: "updated",
      font_size: 20,
      highlight_enabled: false,
    });
  });

  it("reconstructs selected annotation state without React-only persistence", () => {
    const state = freeTextStateFromAnnotation(annotation);
    state.contents = "nama saya محمد 日本";
    state.italic = true;
    state.highlightEnabled = false;

    expect(annotationWithFreeTextState(annotation, state)).toMatchObject({
      contents: "nama saya محمد 日本",
      isItalic: true,
      highlightColor: undefined,
    });
  });

  it("commits multiple applied revisions as one undoable history entry", () => {
    const initial = createInitialHistoryState(new Uint8Array([1, 2, 3]));
    const committed = commitFreeTextSessionHistory(initial, createSession());

    expect(committed.snapshots).toHaveLength(2);
    expect(committed.snapshots[1]?.description).toBe("Edit FreeText");

    const undone = undoHistory(committed);
    expect(undone?.entry.bytes).toEqual(new Uint8Array([1, 2, 3]));
    const redone = redoHistory(undone!.nextState);
    expect(redone?.entry.bytes).toEqual(new Uint8Array([4, 5, 6]));
  });

  it("does not add history for an unchanged session", () => {
    const initial = createInitialHistoryState(new Uint8Array([1, 2, 3]));
    const session = createSession();
    session.dirty = false;

    expect(commitFreeTextSessionHistory(initial, session)).toBe(initial);
  });

  it("cancels back to cloned original bytes and original dirty state", () => {
    const session = createSession();
    session.originalIsModified = true;
    const cancelled = getFreeTextSessionCancelState(session);

    expect(cancelled.bytes).toEqual(new Uint8Array([1, 2, 3]));
    expect(cancelled.bytes).not.toBe(session.originalBytes);
    expect(cancelled.isModified).toBe(true);
    cancelled.bytes[0] = 99;
    expect(session.originalBytes[0]).toBe(1);
  });

  it("advances the native baseline before a stale flush can lose an edit-then-revert", () => {
    const session = createSession();
    session.lastAppliedState = cloneFreeTextState(session.originalState);
    session.revision = 1;
    session.appliedRevision = 0;
    session.refreshedRevision = 0;
    const firstDraft = cloneFreeTextState(session.originalState);
    firstDraft.contents = "temporary value";

    expect(freeTextStateToUpdatePatch(session.lastAppliedState, firstDraft)).toEqual({
      contents: "temporary value",
    });
    recordFreeTextExportedRevision(session, 1, firstDraft, new Uint8Array([7, 8, 9]));

    const revertedDraft = cloneFreeTextState(session.originalState);
    session.draftState = revertedDraft;
    session.revision = 2;
    expect(freeTextStateToUpdatePatch(session.lastAppliedState, revertedDraft)).toEqual({
      contents: session.originalState.contents,
    });
  });

  it("requires byte restoration after any attempted revision, including refresh failure", () => {
    const session = createSession();
    session.revision = 1;
    session.appliedRevision = 0;
    session.refreshedRevision = 0;
    session.latestBytes = session.originalBytes.slice(0);

    expect(freeTextSessionNeedsDocumentRestore(session)).toBe(true);
    recordFreeTextExportedRevision(
      session,
      1,
      session.draftState,
      new Uint8Array([9, 9, 9]),
    );
    const cancelled = getFreeTextSessionCancelState(session);
    expect(cancelled.bytes).toEqual(session.originalBytes);
    expect(cancelled.bytes).not.toEqual(session.latestBytes);
  });

  it("does not reload the document when an untouched session is cancelled", () => {
    const session = createSession();
    session.revision = 0;
    session.appliedRevision = 0;
    session.refreshedRevision = 0;
    session.dirty = false;

    expect(freeTextSessionNeedsDocumentRestore(session)).toBe(false);
  });

  it("retries authoritative refresh after exported bytes survive a refresh failure", async () => {
    const session = createSession();
    session.revision = 1;
    session.appliedRevision = 0;
    session.refreshedRevision = 0;
    recordFreeTextExportedRevision(
      session,
      1,
      session.draftState,
      new Uint8Array([8, 8, 8]),
    );
    expect(freeTextSessionNeedsSync(session)).toBe(true);

    let attempts = 0;
    const refresh = async () => {
      attempts += 1;
      if (attempts === 1) throw new Error("injected PDF.js refresh failure");
      return cloneFreeTextState(session.draftState);
    };

    await expect(refreshFreeTextExportedRevision(session, 1, refresh)).rejects.toThrow(
      "injected PDF.js refresh failure",
    );
    expect(session.refreshedRevision).toBe(0);
    expect(freeTextSessionNeedsSync(session)).toBe(true);

    await expect(refreshFreeTextExportedRevision(session, 1, refresh)).resolves.toEqual(
      session.draftState,
    );
    expect(session.refreshedRevision).toBe(1);
    expect(freeTextSessionNeedsSync(session)).toBe(false);
    expect(session.latestBytes).toEqual(new Uint8Array([8, 8, 8]));
  });
});

/**
 * Component-level coverage for the editor's FreeText synchronization controller
 * (`flushFreeTextSession` calls `drainFreeTextSyncQueue` verbatim).
 *
 * The refresh dependency is injected, so a genuine authoritative-refresh failure
 * can be simulated without a DOM: the failure must propagate, must leave the
 * session unsynchronized, and a subsequent flush must recover.
 */
describe("FreeText sync queue (component controller)", () => {
  it("propagates an injected refresh failure and keeps the session unsynced", async () => {
    const session = createSession();
    session.revision = 1;
    session.appliedRevision = 1;
    session.refreshedRevision = 0; // bytes applied, authoritative refresh pending
    expect(freeTextSessionNeedsSync(session)).toBe(true);

    const order: string[] = [];
    const applyRevision = async () => {
      order.push("apply");
      // Simulates: updateAnnotation + exportIncremental succeeded, then the
      // authoritative UI refresh threw.
      throw new Error("injected refresh failure");
    };

    const deps: FreeTextSyncQueueDeps = {
      getSession: () => session,
      awaitPendingApply: async () => {
        order.push("await");
      },
      applyRevision,
    };

    await expect(drainFreeTextSyncQueue(deps)).rejects.toThrow(
      "injected refresh failure",
    );

    // The queued task drained first, and the failure did not mark the session
    // as refreshed: a later flush must retry rather than accept stale state.
    expect(order).toEqual(["await", "apply"]);
    expect(session.refreshedRevision).toBe(0);
    expect(freeTextSessionNeedsSync(session)).toBe(true);
    expect(session.syncing).toBe(false);
  });

  it("recovers on the next flush after the injected refresh failure", async () => {
    const session = createSession();
    session.revision = 1;
    session.appliedRevision = 1;
    session.refreshedRevision = 0;

    let attempts = 0;
    let latestBytes = new Uint8Array([4, 5, 6]);
    const applyRevision = async (
      _sessionId: number,
      revision: number,
      draft: typeof session.draftState,
    ) => {
      attempts += 1;
      // Native mutation + incremental export always succeed and advance bytes.
      latestBytes = new Uint8Array([9, 9, attempts]);
      recordFreeTextExportedRevision(session, revision, draft, latestBytes);
      if (attempts === 1) {
        throw new Error("injected refresh failure");
      }
      session.refreshedRevision = revision;
    };

    const deps: FreeTextSyncQueueDeps = {
      getSession: () => session,
      awaitPendingApply: async () => undefined,
      applyRevision,
    };

    await expect(drainFreeTextSyncQueue(deps)).rejects.toThrow("injected refresh failure");
    expect(freeTextSessionNeedsSync(session)).toBe(true);
    // Exported bytes were not lost by the refresh failure.
    expect(session.latestBytes).toEqual(new Uint8Array([9, 9, 1]));

    await expect(drainFreeTextSyncQueue(deps)).resolves.toBeUndefined();
    expect(attempts).toBe(2);
    expect(freeTextSessionNeedsSync(session)).toBe(false);
    expect(session.refreshedRevision).toBe(session.revision);
  });

  it("stops instead of spinning when a revision makes no progress", async () => {
    const session = createSession();
    session.revision = 2;
    session.appliedRevision = 1;
    session.refreshedRevision = 1;

    let calls = 0;
    const deps: FreeTextSyncQueueDeps = {
      getSession: () => session,
      awaitPendingApply: async () => undefined,
      // Applies nothing: revision/applied/refreshed all stay exactly as they were.
      applyRevision: async () => {
        calls += 1;
      },
    };

    await expect(drainFreeTextSyncQueue(deps)).rejects.toThrow(
      "FreeText revision did not make progress.",
    );
    expect(calls).toBe(1);
  });

  it("does nothing when the session is already synchronized or closed", async () => {
    const synced = createSession(); // revision 3 / applied 3 / refreshed 3
    let calls = 0;
    const deps: FreeTextSyncQueueDeps = {
      getSession: () => synced,
      awaitPendingApply: async () => undefined,
      applyRevision: async () => {
        calls += 1;
      },
    };

    await drainFreeTextSyncQueue(deps);
    expect(calls).toBe(0);

    await drainFreeTextSyncQueue({ ...deps, getSession: () => null });
    expect(calls).toBe(0);
  });
});
