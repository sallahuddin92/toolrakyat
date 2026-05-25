import { describe, it, expect } from "vitest";

import { formatBytes } from "./format";

describe("formatBytes", () => {
  it("formats bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1024)).toBe("1 KB");
  });
});
