/**
 * Human-friendly error translation for StarPDF operations.
 * Maps typed engine error codes and conditions to clear, actionable user messages.
 */

export interface FriendlyErrorResult {
  userMessage: string;
  technicalDetails?: string;
  isUnsupported: boolean;
  canRetry: boolean;
}

export function formatPdfErrorMessage(error: unknown): FriendlyErrorResult {
  const rawMessage = error instanceof Error ? error.message : String(error);

  // 1. Encryption & Security
  if (
    rawMessage.includes("encrypted") ||
    rawMessage.includes("STANDARD_SECURITY_DETECTED") ||
    rawMessage.includes("PUBLIC_KEY_SECURITY_DETECTED") ||
    rawMessage.includes("security-handler")
  ) {
    return {
      userMessage:
        "This PDF is encrypted with an unsupported security handler or password. StarPDF does not decrypt or bypass document security.",
      technicalDetails: rawMessage,
      isUnsupported: true,
      canRetry: false,
    };
  }

  // 2. Font / Encoding & Glyph limitations
  if (
    rawMessage.includes("UNREPRESENTABLE") ||
    rawMessage.includes("cannot be encoded by font") ||
    rawMessage.includes("missing glyph") ||
    rawMessage.includes("No compatible font found")
  ) {
    return {
      userMessage:
        "The font used for this text doesn't support one or more of the characters you typed, and no compatible fallback could be safely matched. Your document was left unchanged.",
      technicalDetails: "StarPDF refused the edit to avoid rendering missing or corrupted glyphs.",
      isUnsupported: true,
      canRetry: false,
    };
  }

  if (
    rawMessage.includes("UNSUPPORTED_FONT_ENCODING") ||
    rawMessage.includes("unsupported font encoding") ||
    rawMessage.includes("ToUnicode")
  ) {
    return {
      userMessage:
        "That text uses a specialized font encoding that StarPDF cannot safely rewrite without risking layout distortion.",
      technicalDetails: rawMessage,
      isUnsupported: true,
      canRetry: false,
    };
  }

  // 3. Text Move Dependencies & Layout Positioning
  if (
    rawMessage.includes("TEXT_MOVE_DOWNSTREAM_DEPENDENT_TEXT") ||
    rawMessage.includes("TEXT_MOVE_SHARED_POSITIONER")
  ) {
    return {
      userMessage:
        "This text can’t be moved independently because nearby text shares its positioning. Your document was left unchanged.",
      technicalDetails: "StarPDF refused the move to prevent other text from shifting.",
      isUnsupported: true,
      canRetry: false,
    };
  }

  if (
    rawMessage.includes("TEXT_MOVE_STATE_DEPENDENT") ||
    rawMessage.includes("TEXT_MOVE_UNSUPPORTED_POSITIONER") ||
    rawMessage.includes("TEXT_MOVE_NO_EXPLICIT_POSITIONER") ||
    rawMessage.includes("TEXT_MOVE_OUTSIDE_TEXT_BLOCK") ||
    rawMessage.includes("TEXT_MOVE_SINGULAR_TRANSFORM")
  ) {
    return {
      userMessage:
        "This text can’t be safely moved in place because its position depends on complex formatting. Your document was left unchanged.",
      technicalDetails: "StarPDF refused the move to protect document layout integrity.",
      isUnsupported: true,
      canRetry: false,
    };
  }

  // 4. Layout / Spacing Dependencies
  if (
    rawMessage.includes("depends on the spacing of this text run") ||
    rawMessage.includes("UNSUPPORTED_LAYOUT") ||
    rawMessage.includes("UnsupportedLayout")
  ) {
    return {
      userMessage: "This text can't be safely edited in place.",
      technicalDetails: "Other text in this PDF depends on the spacing of this text run.",
      isUnsupported: true,
      canRetry: false,
    };
  }

  // 4. Complex Script / Vertical Writing
  if (
    rawMessage.includes("UNSUPPORTED_COMPLEX_SCRIPT") ||
    rawMessage.includes("UNSUPPORTED_VERTICAL_WRITING")
  ) {

    return {
      userMessage:
        "This text uses complex script shaping or vertical writing layout, which is currently read-only.",
      technicalDetails: rawMessage,
      isUnsupported: true,
      canRetry: false,
    };
  }

  // 4. Shared or ambiguous objects
  if (
    rawMessage.includes("shared") ||
    rawMessage.includes("ambiguous shared content") ||
    rawMessage.includes("Shared XObject")
  ) {
    return {
      userMessage:
        "This object is shared across multiple pages or content streams in a structure that cannot be isolated safely.",
      technicalDetails: rawMessage,
      isUnsupported: true,
      canRetry: false,
    };
  }

  // 5. Page Geometry / MediaBox
  if (
    rawMessage.includes("MediaBox") ||
    rawMessage.includes("geometry") ||
    rawMessage.includes("no direct or inherited")
  ) {
    return {
      userMessage:
        "Page dimensions or geometry are missing or invalid in this document.",
      technicalDetails: rawMessage,
      isUnsupported: true,
      canRetry: false,
    };
  }

  // 6. Malformed Syntax
  if (
    rawMessage.includes("InvalidSyntax") ||
    rawMessage.includes("malformed") ||
    rawMessage.includes("corrupt")
  ) {
    return {
      userMessage:
        "The PDF contains malformed syntax that cannot be safely reconstructed.",
      technicalDetails: rawMessage,
      isUnsupported: true,
      canRetry: false,
    };
  }

  // Generic fallback
  return {
    userMessage: rawMessage || "An unexpected error occurred while processing the PDF.",
    technicalDetails: rawMessage,
    isUnsupported: false,
    canRetry: true,
  };
}
