import { describe, it, expect, beforeEach } from "vitest";
import {
  getRecentDocuments,
  addRecentDocument,
  removeRecentDocument,
  clearRecentDocuments,
} from "./recentFilesStore";

describe("SmartPDF Recent Files Store", () => {
  beforeEach(async () => {
    await clearRecentDocuments();
  });

  it("returns empty list initially", async () => {
    const recents = await getRecentDocuments();
    expect(recents).toEqual([]);
  });

  it("adds and retrieves a recent document", async () => {
    const dummyBytes = new Uint8Array([1, 2, 3, 4, 5]);
    const added = await addRecentDocument({
      name: "contract.pdf",
      bytes: dummyBytes,
      pageCount: 3,
    });

    expect(added.name).toBe("contract.pdf");
    expect(added.sizeBytes).toBe(5);
    expect(added.pageCount).toBe(3);

    const recents = await getRecentDocuments();
    expect(recents.length).toBe(1);
    expect(recents[0].name).toBe("contract.pdf");
  });

  it("orders recent documents by lastOpened descending", async () => {
    const bytesA = new Uint8Array([1]);
    const bytesB = new Uint8Array([2]);

    await addRecentDocument({ name: "first.pdf", bytes: bytesA });
    // Slight pause to guarantee distinct timestamp
    await new Promise((r) => setTimeout(r, 10));
    await addRecentDocument({ name: "second.pdf", bytes: bytesB });

    const recents = await getRecentDocuments();
    expect(recents.length).toBe(2);
    expect(recents[0].name).toBe("second.pdf");
    expect(recents[1].name).toBe("first.pdf");
  });

  it("removes a specific document by id", async () => {
    const added = await addRecentDocument({
      name: "to-remove.pdf",
      bytes: new Uint8Array([1, 2]),
    });

    let recents = await getRecentDocuments();
    expect(recents.some((d) => d.id === added.id)).toBe(true);

    await removeRecentDocument(added.id);

    recents = await getRecentDocuments();
    expect(recents.some((d) => d.id === added.id)).toBe(false);
  });

  it("clears all recent documents", async () => {
    await addRecentDocument({ name: "a.pdf", bytes: new Uint8Array([1]) });
    await addRecentDocument({ name: "b.pdf", bytes: new Uint8Array([2]) });

    let recents = await getRecentDocuments();
    expect(recents.length).toBe(2);

    await clearRecentDocuments();

    recents = await getRecentDocuments();
    expect(recents.length).toBe(0);
  });
});
