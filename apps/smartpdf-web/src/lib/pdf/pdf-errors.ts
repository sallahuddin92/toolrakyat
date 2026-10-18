export class PdfError extends Error {
  constructor(message: string, public readonly code: string) {
    super(message);
    this.name = "PdfError";
  }
}

export class PdfParseError extends PdfError {
  constructor(message = "Failed to parse PDF document. The file may be corrupt or not a valid PDF.") {
    super(message, "PDF_PARSE_ERROR");
    this.name = "PdfParseError";
  }
}

export class PdfEncryptedError extends PdfError {
  constructor(message = "This PDF document is password-protected or encrypted. Password protection is not supported in Phase 1.") {
    super(message, "PDF_ENCRYPTED_ERROR");
    this.name = "PdfEncryptedError";
  }
}

export class PdfExportError extends PdfError {
  constructor(message = "Failed to export PDF document.") {
    super(message, "PDF_EXPORT_ERROR");
    this.name = "PdfExportError";
  }
}

export class PdfValidationError extends PdfError {
  constructor(message = "Exported PDF validation failed. The generated document is unreadable.") {
    super(message, "PDF_VALIDATION_ERROR");
    this.name = "PdfValidationError";
  }
}
