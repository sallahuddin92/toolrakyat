export interface SmartPdfHistoryEntry {
  bytes: Uint8Array;
  description: string;
  timestamp: number;
}

export interface SmartPdfHistoryState {
  snapshots: SmartPdfHistoryEntry[];
  currentIndex: number;
  maxSnapshots: number;
}

export const MAX_HISTORY_SNAPSHOTS = 25;

export function createInitialHistoryState(
  bytes: Uint8Array,
  description = "Initial document",
  maxSnapshots = MAX_HISTORY_SNAPSHOTS,
): SmartPdfHistoryState {
  return {
    snapshots: [
      {
        bytes: bytes.slice(0),
        description,
        timestamp: Date.now(),
      },
    ],
    currentIndex: 0,
    maxSnapshots,
  };
}

export function pushHistorySnapshot(
  state: SmartPdfHistoryState,
  bytes: Uint8Array,
  description: string,
): SmartPdfHistoryState {
  // Truncate any redo branch
  const activeBranch = state.snapshots.slice(0, state.currentIndex + 1);

  // Append new entry
  const newEntry: SmartPdfHistoryEntry = {
    bytes: bytes.slice(0),
    description,
    timestamp: Date.now(),
  };
  const updatedSnapshots = [...activeBranch, newEntry];

  // Enforce max snapshots limit (discard oldest entries beyond limit)
  const boundedSnapshots =
    updatedSnapshots.length > state.maxSnapshots
      ? updatedSnapshots.slice(updatedSnapshots.length - state.maxSnapshots)
      : updatedSnapshots;

  const nextIndex = boundedSnapshots.length - 1;

  return {
    ...state,
    snapshots: boundedSnapshots,
    currentIndex: nextIndex,
  };
}

export function canUndo(state: SmartPdfHistoryState): boolean {
  return state.currentIndex > 0;
}

export function canRedo(state: SmartPdfHistoryState): boolean {
  return state.currentIndex < state.snapshots.length - 1;
}

export function undoHistory(
  state: SmartPdfHistoryState,
): { nextState: SmartPdfHistoryState; entry: SmartPdfHistoryEntry } | null {
  if (!canUndo(state)) return null;
  const nextIndex = state.currentIndex - 1;
  const entry = state.snapshots[nextIndex];
  return {
    nextState: {
      ...state,
      currentIndex: nextIndex,
    },
    entry,
  };
}

export function redoHistory(
  state: SmartPdfHistoryState,
): { nextState: SmartPdfHistoryState; entry: SmartPdfHistoryEntry } | null {
  if (!canRedo(state)) return null;
  const nextIndex = state.currentIndex + 1;
  const entry = state.snapshots[nextIndex];
  return {
    nextState: {
      ...state,
      currentIndex: nextIndex,
    },
    entry,
  };
}
