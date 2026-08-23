import type { SmartPdfCommand, SmartPdfCommandContext, SmartPdfCommandResult } from "./types";
import {
  runStarPdfPageOperation,
  mergeStarPdfDocuments,
  splitStarPdfDocument,
} from "../starpdf-page-worker-client";
import { StarPdfClient } from "../starpdf-client";

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
    if (this.fromIndex === this.toIndex) {
      return {
        bytes: sourceBytes,
        nextPage: this.fromIndex + 1,
        clearSelection: true,
        message: "Page position unchanged.",
      };
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

export class ReorderPagesCommand implements SmartPdfCommand {
  readonly id = "page.reorder";
  readonly label: string;
  readonly isMutating = true;

  constructor(
    public readonly newOrder: number[], // 0-indexed permutation of all pages [0..pageCount-1]
    public readonly targetActivePage?: number, // 1-indexed next active page
  ) {
    this.label = "Pages reordered.";
  }

  static fromMoveBlock(
    pageCount: number,
    movingIndices: number[], // 0-indexed
    targetIndex: number, // 0-indexed target page to place relative to
    placeBefore: boolean,
  ): ReorderPagesCommand {
    if (movingIndices.length === 0) {
      throw new Error("No pages selected to reorder.");
    }
    const movingSet = new Set(movingIndices);
    const movingSorted = movingIndices.slice().sort((a, b) => a - b);
    const remaining = Array.from({ length: pageCount }, (_, i) => i).filter((i) => !movingSet.has(i));

    let insertAt = remaining.indexOf(targetIndex);
    if (insertAt === -1) {
      insertAt = targetIndex >= pageCount ? remaining.length : 0;
    } else if (!placeBefore) {
      insertAt += 1;
    }

    const newOrder = [
      ...remaining.slice(0, insertAt),
      ...movingSorted,
      ...remaining.slice(insertAt),
    ];

    const targetActivePage = insertAt + 1;
    return new ReorderPagesCommand(newOrder, targetActivePage);
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, pageCount } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (this.newOrder.length !== pageCount) {
      throw new Error(`Invalid reorder list length (${this.newOrder.length} !== ${pageCount}).`);
    }

    // Check for 0-change (identity permutation)
    const isIdentity = this.newOrder.every((val, idx) => val === idx);
    if (isIdentity) {
      return {
        bytes: sourceBytes,
        nextPage: this.targetActivePage ?? context.currentPage,
        clearSelection: true,
        message: "Page order unchanged.",
      };
    }

    const pageSources = this.newOrder.map((pageIndex) => ({
      documentIndex: 0,
      pageIndex,
    }));

    const output = await mergeStarPdfDocuments([sourceBytes], pageSources);
    const targetPage = this.targetActivePage ?? Math.max(1, Math.min(pageCount, (this.newOrder.indexOf(0) || 0) + 1));

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

export class DuplicatePagesBatchCommand implements SmartPdfCommand {
  readonly id = "page.duplicate-batch";
  readonly label: string;
  readonly isMutating = true;

  constructor(public readonly pageIndices: number[]) {
    // 0-indexed page indices
    this.label = `Duplicated ${pageIndices.length} page(s).`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, pageCount } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (this.pageIndices.length === 0) {
      throw new Error("No pages selected to duplicate.");
    }

    const sorted = this.pageIndices.slice().sort((a, b) => a - b);
    for (const idx of sorted) {
      if (idx < 0 || idx >= pageCount) {
        throw new Error(`Page index ${idx} out of bounds.`);
      }
    }

    // Insert duplicate copies immediately after the last selected page
    const maxIndex = Math.max(...sorted);
    const newSequence: { documentIndex: number; pageIndex: number }[] = [];

    for (let i = 0; i < pageCount; i++) {
      newSequence.push({ documentIndex: 0, pageIndex: i });
      if (i === maxIndex) {
        for (const dupIdx of sorted) {
          newSequence.push({ documentIndex: 0, pageIndex: dupIdx });
        }
      }
    }

    const output = await mergeStarPdfDocuments([sourceBytes], newSequence);
    const targetPage = maxIndex + 2;

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

export class DeletePagesBatchCommand implements SmartPdfCommand {
  readonly id = "page.delete-batch";
  readonly label: string;
  readonly isMutating = true;

  constructor(public readonly pageIndices: number[]) {
    // 0-indexed page indices
    this.label = `Deleted ${pageIndices.length} page(s).`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, pageCount, currentPage } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (this.pageIndices.length === 0) {
      throw new Error("No pages selected to delete.");
    }
    if (this.pageIndices.length >= pageCount) {
      throw new Error("Cannot delete all pages in a document. At least one page must remain.");
    }

    const deletedSet = new Set(this.pageIndices);
    const remaining = Array.from({ length: pageCount }, (_, i) => i).filter((i) => !deletedSet.has(i));

    if (remaining.length === 0) {
      throw new Error("Cannot delete all pages in a document.");
    }

    const pageSources = remaining.map((pageIndex) => ({
      documentIndex: 0,
      pageIndex,
    }));

    const output = await mergeStarPdfDocuments([sourceBytes], pageSources);

    // Calculate nearest remaining active page (1-indexed)
    const currentZeroIdx = currentPage - 1;
    let nextZeroIdx = remaining.findIndex((idx) => idx >= currentZeroIdx);
    if (nextZeroIdx === -1) {
      nextZeroIdx = remaining.length - 1;
    }
    const targetPage = nextZeroIdx + 1;

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
    public readonly pageIndex: number, // 0-indexed insertion point
    public readonly width = 612,
    public readonly height = 792,
    public readonly rotation: 0 | 90 | 180 | 270 = 0,
    public readonly position: "before" | "after" = "after",
  ) {}

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, pageCount } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }

    const targetInsertIndex = this.position === "before" ? this.pageIndex : this.pageIndex + 1;
    const clampedIndex = Math.max(0, Math.min(pageCount, targetInsertIndex));

    const output = await runStarPdfPageOperation(sourceBytes, {
      type: "insertBlankPage",
      pageIndex: clampedIndex,
      width: this.width,
      height: this.height,
      rotation: this.rotation,
    });

    const targetPage = clampedIndex + 1;

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

export class InsertImportedPagesCommand implements SmartPdfCommand {
  readonly id = "document.import-pages";
  readonly label: string;
  readonly isMutating = true;

  constructor(
    public readonly importedBytes: Uint8Array,
    public readonly importedPageIndices: number[], // 0-indexed
    public readonly position: "start" | "end" | "before" | "after",
    public readonly activePageIndex: number, // 0-indexed current page in primary doc
    public readonly sourceFilename?: string,
  ) {
    this.label = `Imported ${importedPageIndices.length} page(s)${sourceFilename ? ` from ${sourceFilename}` : ""}.`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, pageCount } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (!this.importedBytes || this.importedBytes.length === 0) {
      throw new Error("No imported document bytes provided.");
    }

    // Inspect and validate imported PDF locally
    let handle;
    let importedTotalPages = 0;
    try {
      handle = await StarPdfClient.open(this.importedBytes);
      const secInfo = await handle.getSecurityInfo();
      if (secInfo.encryption_state !== "NOT_ENCRYPTED") {
        await handle.close();
        const err = new Error("Source document is password-protected or encrypted and cannot be merged.");
        err.name = "ENCRYPTED_DOCUMENT";
        throw err;
      }
      importedTotalPages = await handle.getPageCount();
      await handle.close();
    } catch (e: unknown) {
      if (e instanceof Error && e.name === "ENCRYPTED_DOCUMENT") {
        throw e;
      }
      throw new Error(`Failed to validate imported PDF: ${e instanceof Error ? e.message : String(e)}`);
    }

    const pagesToImport =
      this.importedPageIndices.length > 0
        ? this.importedPageIndices.filter((idx) => idx >= 0 && idx < importedTotalPages)
        : Array.from({ length: importedTotalPages }, (_, i) => i);

    if (pagesToImport.length === 0) {
      throw new Error("No valid pages selected for import.");
    }

    let insertAt = pageCount;
    if (this.position === "start") {
      insertAt = 0;
    } else if (this.position === "end") {
      insertAt = pageCount;
    } else if (this.position === "before") {
      insertAt = Math.max(0, Math.min(pageCount, this.activePageIndex));
    } else if (this.position === "after") {
      insertAt = Math.max(0, Math.min(pageCount, this.activePageIndex + 1));
    }

    const pageSources: { documentIndex: number; pageIndex: number }[] = [];

    // Pages from doc 0 before insert point
    for (let i = 0; i < insertAt; i++) {
      pageSources.push({ documentIndex: 0, pageIndex: i });
    }
    // Imported pages from doc 1
    for (const pageIdx of pagesToImport) {
      pageSources.push({ documentIndex: 1, pageIndex: pageIdx });
    }
    // Remaining pages from doc 0
    for (let i = insertAt; i < pageCount; i++) {
      pageSources.push({ documentIndex: 0, pageIndex: i });
    }

    const output = await mergeStarPdfDocuments([sourceBytes, this.importedBytes], pageSources);
    const targetPage = insertAt + 1;

    return {
      bytes: output,
      nextPage: targetPage,
      clearSelection: true,
      message: this.label,
    };
  }
}

export class SplitDocumentCommand implements SmartPdfCommand {
  readonly id = "document.split";
  readonly label: string;
  readonly isMutating = false;

  constructor(
    public readonly ranges: { start: number; endExclusive: number }[], // 0-indexed
  ) {
    this.label = `Split into ${ranges.length} part(s)`;
  }

  async execute(context: SmartPdfCommandContext): Promise<SmartPdfCommandResult> {
    const { sourceBytes, filename } = context;
    if (!sourceBytes) {
      throw new Error("No source document loaded.");
    }
    if (this.ranges.length === 0) {
      throw new Error("No split ranges specified.");
    }

    const outputs = await splitStarPdfDocument(sourceBytes, this.ranges);
    if (outputs.length === 0) {
      throw new Error("Split produced no outputs.");
    }

    const baseName = filename.replace(/\.pdf$/i, "");
    // Return first split part or download
    const firstOutput = outputs[0];
    const downloadFilename = `${baseName}-part1.pdf`;

    return {
      download: {
        filename: downloadFilename,
        bytes: firstOutput,
      },
      message: `Split into ${outputs.length} part(s) successfully.`,
    };
  }
}
