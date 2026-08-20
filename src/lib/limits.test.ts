import { describe, it, expect } from "vitest";
import { LIMITS } from "./limits";

describe("LIMITS configuration", () => {
  it("provides positive numerical values for all default limits", () => {
    expect(LIMITS.GLOBAL_MAX_FILE_SIZE_MB).toBeGreaterThan(0);
    expect(LIMITS.RATE_LIMIT_MAX).toBeGreaterThan(0);
    expect(LIMITS.RATE_LIMIT_BURST).toBeGreaterThan(0);
    expect(LIMITS.TEMP_FILE_MAX_AGE_MINUTES).toBeGreaterThan(0);
  });

  it("matches documented security baseline defaults", () => {
    expect(LIMITS.GLOBAL_MAX_FILE_SIZE_MB).toBe(20);
    expect(LIMITS.RATE_LIMIT_MAX).toBe(20);
    expect(LIMITS.RATE_LIMIT_BURST).toBe(10);
    expect(LIMITS.TEMP_FILE_MAX_AGE_MINUTES).toBe(30);
  });
});
