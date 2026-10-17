import type { RecentDocumentMeta } from "../types";

const RECENT_META_STORAGE_KEY = "smartpdf:recent_documents";
const MAX_RECENT_ITEMS = 10;
const DB_NAME = "smartpdf_cache_db";
const STORE_NAME = "document_bytes";

// In-memory fallback if IndexedDB or localStorage is unavailable
const memoryCache = new Map<string, Uint8Array>();
const memoryStorage: Record<string, string> = {};

interface SimpleStorage {
  getItem: (key: string) => string | null;
  setItem: (key: string, val: string) => void;
  removeItem: (key: string) => void;
}

function getStorage(): SimpleStorage {
  if (typeof window !== "undefined") {
    try {
      if (window.localStorage) {
        return window.localStorage;
      }
    } catch {
      // Fallback to memoryStorage
    }
  }
  return {
    getItem: (key: string) => memoryStorage[key] ?? null,
    setItem: (key: string, val: string) => {
      memoryStorage[key] = val;
    },
    removeItem: (key: string) => {
      delete memoryStorage[key];
    },
  };
}

function openCacheDatabase(): Promise<IDBDatabase | null> {
  if (typeof window === "undefined" || !("indexedDB" in window)) {
    return Promise.resolve(null);
  }

  return new Promise((resolve) => {
    try {
      const request = window.indexedDB.open(DB_NAME, 1);
      request.onupgradeneeded = () => {
        const db = request.result;
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          db.createObjectStore(STORE_NAME);
        }
      };
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => resolve(null);
    } catch {
      resolve(null);
    }
  });
}

export async function getRecentDocuments(): Promise<RecentDocumentMeta[]> {
  const storage = getStorage();
  if (!storage) return [];

  try {
    const raw = storage.getItem(RECENT_META_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as RecentDocumentMeta[];
    if (Array.isArray(parsed)) {
      return parsed.sort((a, b) => b.lastOpened - a.lastOpened);
    }
  } catch {
    // Return empty on corrupt JSON
  }
  return [];
}

export async function addRecentDocument(doc: {
  name: string;
  bytes: Uint8Array;
  pageCount?: number;
}): Promise<RecentDocumentMeta> {
  const id = `doc-${Date.now()}-${Math.random().toString(36).substring(2, 7)}`;
  const item: RecentDocumentMeta = {
    id,
    name: doc.name,
    sizeBytes: doc.bytes.byteLength,
    lastOpened: Date.now(),
    pageCount: doc.pageCount,
  };

  // 1. Store metadata
  const existing = await getRecentDocuments();
  const filtered = existing.filter((d) => d.name !== doc.name);
  const updated = [item, ...filtered].slice(0, MAX_RECENT_ITEMS);

  const storage = getStorage();
  if (storage) {
    try {
      storage.setItem(RECENT_META_STORAGE_KEY, JSON.stringify(updated));
    } catch {
      // Ignore storage quota errors
    }
  }

  // 2. Cache bytes in IndexedDB or memory (up to 20MB per document)
  if (doc.bytes.byteLength <= 20 * 1024 * 1024) {
    memoryCache.set(id, doc.bytes);
    const db = await openCacheDatabase();
    if (db) {
      try {
        const tx = db.transaction(STORE_NAME, "readwrite");
        tx.objectStore(STORE_NAME).put(doc.bytes, id);
      } catch {
        // Ignore cache failure
      }
    }
  }

  return item;
}

export async function getRecentDocumentBytes(id: string): Promise<Uint8Array | null> {
  if (memoryCache.has(id)) {
    return memoryCache.get(id) || null;
  }

  const db = await openCacheDatabase();
  if (!db) return null;

  return new Promise((resolve) => {
    try {
      const tx = db.transaction(STORE_NAME, "readonly");
      const request = tx.objectStore(STORE_NAME).get(id);
      request.onsuccess = () => {
        if (request.result instanceof Uint8Array) {
          memoryCache.set(id, request.result);
          resolve(request.result);
        } else {
          resolve(null);
        }
      };
      request.onerror = () => resolve(null);
    } catch {
      resolve(null);
    }
  });
}

export async function removeRecentDocument(id: string): Promise<void> {
  const existing = await getRecentDocuments();
  const updated = existing.filter((d) => d.id !== id);

  const storage = getStorage();
  if (storage) {
    storage.setItem(RECENT_META_STORAGE_KEY, JSON.stringify(updated));
  }

  memoryCache.delete(id);

  const db = await openCacheDatabase();
  if (db) {
    try {
      const tx = db.transaction(STORE_NAME, "readwrite");
      tx.objectStore(STORE_NAME).delete(id);
    } catch {
      // Ignore
    }
  }
}

export async function clearRecentDocuments(): Promise<void> {
  const storage = getStorage();
  if (storage) {
    storage.removeItem(RECENT_META_STORAGE_KEY);
  }

  memoryCache.clear();

  const db = await openCacheDatabase();
  if (db) {
    try {
      const tx = db.transaction(STORE_NAME, "readwrite");
      tx.objectStore(STORE_NAME).clear();
    } catch {
      // Ignore
    }
  }
}
