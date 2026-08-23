import { describe, it, expect } from "vitest";
import { formatPdfErrorMessage } from "./pdf-friendly-errors";

describe("formatPdfErrorMessage", () => {
  it("translates TEXT_MOVE_DOWNSTREAM_DEPENDENT_TEXT into user-friendly message without raw error codes", () => {
    const raw = new Error("Invalid PDF operation: TEXT_MOVE_DOWNSTREAM_DEPENDENT_TEXT");
    const result = formatPdfErrorMessage(raw);

    expect(result.userMessage).toBe(
      "This text can’t be moved independently because nearby text shares its positioning. Your document was left unchanged."
    );
    expect(result.technicalDetails).toBe("StarPDF refused the move to prevent other text from shifting.");
    expect(result.isUnsupported).toBe(true);
    expect(result.canRetry).toBe(false);
    expect(result.userMessage).not.toContain("TEXT_MOVE_DOWNSTREAM_DEPENDENT_TEXT");
    expect(result.userMessage).not.toContain("Invalid PDF operation");
  });

  it("translates TEXT_MOVE_SHARED_POSITIONER into friendly message", () => {
    const raw = new Error("Invalid PDF operation: TEXT_MOVE_SHARED_POSITIONER");
    const result = formatPdfErrorMessage(raw);

    expect(result.userMessage).toBe(
      "This text can’t be moved independently because nearby text shares its positioning. Your document was left unchanged."
    );
    expect(result.technicalDetails).toBe("StarPDF refused the move to prevent other text from shifting.");
    expect(result.userMessage).not.toContain("TEXT_MOVE_SHARED_POSITIONER");
  });

  it("translates complex formatting move errors safely", () => {
    const raw = new Error("Invalid PDF operation: TEXT_MOVE_STATE_DEPENDENT");
    const result = formatPdfErrorMessage(raw);

    expect(result.userMessage).toBe(
      "This text can’t be safely moved in place because its position depends on complex formatting. Your document was left unchanged."
    );
    expect(result.technicalDetails).toBe("StarPDF refused the move to protect document layout integrity.");
  });

  it("translates font encoding errors", () => {
    const raw = new Error("UNSUPPORTED_FONT_ENCODING");
    const result = formatPdfErrorMessage(raw);

    expect(result.userMessage).toContain("specialized font encoding");
  });

  it("translates encryption errors", () => {
    const raw = new Error("STANDARD_SECURITY_DETECTED");
    const result = formatPdfErrorMessage(raw);

    expect(result.userMessage).toContain("encrypted");
  });
});
