import { describe, it, expect } from "vitest";

import { validateUploadedFile } from "./file-validation";

describe("validateUploadedFile", () => {
  it("accepts allowed type via browser-provided MIME fallback", async () => {
    const f = new File([new Uint8Array([0x25, 0x50, 0x44, 0x46])], "a.pdf", {
      type: "application/pdf",
    });
    const res = await validateUploadedFile({
      file: f,
      allowedTypes: ["application/pdf"],
      maxSizeMB: 1,
    });
    expect(res.ok).toBe(true);
  });

  it("rejects invalid extension", async () => {
    const f = new File([new Uint8Array([0x25, 0x50, 0x44, 0x46])], "a.png", {
      type: "application/pdf",
    });
    const res = await validateUploadedFile({
      file: f,
      allowedTypes: ["application/pdf"],
      maxSizeMB: 1,
    });
    expect(res.ok).toBe(false);
  });

  it("rejects oversized file", async () => {
    const f = new File([new Uint8Array(2 * 1024 * 1024)], "large.pdf", {
      type: "application/pdf",
    });
    const res = await validateUploadedFile({
      file: f,
      allowedTypes: ["application/pdf"],
      maxSizeMB: 1,
    });
    if (!res.ok) {
      expect(res.error).toContain("too large");
    } else {
      throw new Error("Validation should have failed");
    }
  });
});
