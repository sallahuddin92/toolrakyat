import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";
import { runStarPdfPageOperation } from "../starpdf-page-worker-client";

export class MovePageCommand implements SmartPdfCommand {
  readonly id = "page.move";
  readonly label: string;
  readonly isMutating = true;

  constructor(
    public readonly fromIndex: number, // 0-indexed
    public readonly toIndex: number, // 0-indexed
  ) {
    this.label = fromIndex < toIndex ? "Page moved right." : "Page moved left.";
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, pageCount } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (this.fromIndex < 0 || this.fromIndex >= pageCount || this.toIndex < 0 || this.toIndex >= pageCount) {
      throw new Error("Target page index out of bounds.");
    }

    const output = await runStarPdfPageOperation(sourceBytes, {
      type: "movePage",
      fromIndex: this.fromIndex,
      toIndex: this.toIndex,
    });

    // Active page follows moved page (1-indexed)
    const targetPage = this.toIndex + 1;

    return {
      bytes: output,
      nextPage: targetPage,
      clearSelection: true,
      message: this.label,
    };
  }
}

export class DuplicatePageCommand implements SmartPdfCommand {
  readonly id = "page.duplicate";
  readonly label = "Page duplicated.";
  readonly isMutating = true;

  constructor(public readonly pageIndex: number) {} // 0-indexed

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, pageCount } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (this.pageIndex < 0 || this.pageIndex >= pageCount) {
      throw new Error("Page index out of bounds.");
    }

    const destinationIndex = this.pageIndex + 1;
    const output = await runStarPdfPageOperation(sourceBytes, {
      type: "duplicatePage",
      pageIndex: this.pageIndex,
      destinationIndex,
    });

    const targetPage = destinationIndex + 1;

    return {
      bytes: output,
      nextPage: targetPage,
      clearSelection: true,
      message: this.label,
    };
  }
}

export class DeletePageCommand implements SmartPdfCommand {
  readonly id = "page.delete";
  readonly label = "Page deleted.";
  readonly isMutating = true;

  constructor(public readonly pageIndex: number) {} // 0-indexed

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, pageCount, currentPage } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (pageCount <= 1) {
      throw new Error("Cannot delete the only page in a document.");
    }
    if (this.pageIndex < 0 || this.pageIndex >= pageCount) {
      throw new Error("Page index out of bounds.");
    }

    const output = await runStarPdfPageOperation(sourceBytes, {
      type: "deletePage",
      pageIndex: this.pageIndex,
    });

    // Clamp current page atomically (1-indexed)
    const newPageCount = pageCount - 1;
    const targetPage = currentPage > newPageCount ? Math.max(1, newPageCount) : currentPage;

    return {
      bytes: output,
      nextPage: targetPage,
      clearSelection: true,
      message: this.label,
    };
  }
}

export class InsertBlankPageCommand implements SmartPdfCommand {
  readonly id = "page.insert-blank";
  readonly label = "Blank page added.";
  readonly isMutating = true;

  constructor(
    public readonly pageIndex: number, // 0-indexed (insert after)
    public readonly width = 612,
    public readonly height = 792,
    public readonly rotation: 0 | 90 | 180 | 270 = 0,
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }

    const output = await runStarPdfPageOperation(sourceBytes, {
      type: "insertBlankPage",
      pageIndex: this.pageIndex,
      width: this.width,
      height: this.height,
      rotation: this.rotation,
    });

    const targetPage = this.pageIndex + 1;

    return {
      bytes: output,
      nextPage: targetPage,
      clearSelection: true,
      message: this.label,
    };
  }
}

export class ExtractPagesCommand implements SmartPdfCommand {
  readonly id = "page.extract";
  readonly label: string;
  readonly isMutating = false; // Extract does not mutate the current in-memory workspace document

  constructor(public readonly pageIndices: number[]) {
    this.label = `Extract ${pageIndices.length} page(s)`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, filename } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (this.pageIndices.length === 0) {
      throw new Error("No pages selected for extraction.");
    }

    const output = await runStarPdfPageOperation(sourceBytes, {
      type: "extractPages",
      pageIndices: this.pageIndices,
    });

    const baseName = filename.replace(/\.pdf$/i, "");
    const downloadFilename = `${baseName}-extracted.pdf`;

    return {
      download: {
        filename: downloadFilename,
        bytes: output,
      },
      message: `Extracted ${this.pageIndices.length} page(s) successfully.`,
    };
  }
}
