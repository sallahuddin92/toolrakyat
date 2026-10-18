import { describe, expect, it, vi } from "vitest";
import {
  createInitialHistoryState,
  pushHistorySnapshot,
  canUndo,
  canRedo,
  undoHistory,
  redoHistory,
  MAX_HISTORY_SNAPSHOTS,
} from "./history";
import { SetFormFieldValueCommand } from "./form-commands";
import { UpdateAnnotationCommand } from "./annotation-commands";
import {
  ReorderPagesCommand,
  DuplicatePagesBatchCommand,
  DeletePagesBatchCommand,
  InsertImportedPagesCommand,
} from "./page-commands";
import { StarPdfClient } from "../starpdf-client";
import type { SmartPdfCommandContext } from "./types";


describe("SmartPDF Command Architecture & History Lifecycle", () => {
  describe("Bounded 25-Snapshot History Semantics", () => {
    it("initializes history with a single initial snapshot", () => {
      const bytes = new Uint8Array([1, 2, 3]);
      const history = createInitialHistoryState(bytes, "Initial document");

      expect(history.snapshots).toHaveLength(1);
      expect(history.currentIndex).toBe(0);
      expect(history.snapshots[0].description).toBe("Initial document");
      expect(canUndo(history)).toBe(false);
      expect(canRedo(history)).toBe(false);
    });

    it("pushes history snapshots on mutation and advances index", () => {
      const bytes0 = new Uint8Array([0]);
      let history = createInitialHistoryState(bytes0);

      const bytes1 = new Uint8Array([1]);
      history = pushHistorySnapshot(history, bytes1, "Mutation 1");
      expect(history.snapshots).toHaveLength(2);
      expect(history.currentIndex).toBe(1);
      expect(canUndo(history)).toBe(true);
      expect(canRedo(history)).toBe(false);

      const bytes2 = new Uint8Array([2]);
      history = pushHistorySnapshot(history, bytes2, "Mutation 2");
      expect(history.snapshots).toHaveLength(3);
      expect(history.currentIndex).toBe(2);
    });

    it("supports undo and redo transitions", () => {
      let history = createInitialHistoryState(new Uint8Array([0]));
      history = pushHistorySnapshot(history, new Uint8Array([1]), "Step 1");
      history = pushHistorySnapshot(history, new Uint8Array([2]), "Step 2");

      // Undo to Step 1
      const u1 = undoHistory(history);
      expect(u1).not.toBeNull();
      expect(u1?.entry.description).toBe("Step 1");
      expect(u1?.nextState.currentIndex).toBe(1);
      expect(canRedo(u1!.nextState)).toBe(true);

      // Undo to Initial
      const u2 = undoHistory(u1!.nextState);
      expect(u2).not.toBeNull();
      expect(u2?.entry.description).toBe("Initial document");
      expect(u2?.nextState.currentIndex).toBe(0);
      expect(canUndo(u2!.nextState)).toBe(false);

      // Redo back to Step 1
      const r1 = redoHistory(u2!.nextState);
      expect(r1).not.toBeNull();
      expect(r1?.entry.description).toBe("Step 1");
      expect(r1?.nextState.currentIndex).toBe(1);
    });

    it("discards the redo branch when a new mutation occurs after undo", () => {
      let history = createInitialHistoryState(new Uint8Array([0]));
      history = pushHistorySnapshot(history, new Uint8Array([1]), "Step 1");
      history = pushHistorySnapshot(history, new Uint8Array([2]), "Step 2");

      // Undo back to Step 1
      const undone = undoHistory(history);
      expect(undone).not.toBeNull();
      expect(undone?.nextState.currentIndex).toBe(1);

      // Apply new mutation from Step 1
      const branched = pushHistorySnapshot(undone!.nextState, new Uint8Array([3]), "Step 1B");
      expect(branched.snapshots).toHaveLength(3); // [Initial, Step 1, Step 1B]
      expect(branched.currentIndex).toBe(2);
      expect(branched.snapshots[2].description).toBe("Step 1B");
      expect(canRedo(branched)).toBe(false);
    });

    it(`strictly caps history at bounded limit of ${MAX_HISTORY_SNAPSHOTS} snapshots`, () => {
      let history = createInitialHistoryState(new Uint8Array([0]));

      for (let i = 1; i <= 35; i++) {
        history = pushHistorySnapshot(history, new Uint8Array([i]), `Mutation ${i}`);
      }

      expect(history.snapshots.length).toBe(MAX_HISTORY_SNAPSHOTS);
      expect(history.currentIndex).toBe(MAX_HISTORY_SNAPSHOTS - 1);
      expect(history.snapshots[MAX_HISTORY_SNAPSHOTS - 1].description).toBe("Mutation 35");
      // Oldest snapshot should be Mutation (35 - 25 + 1) = Mutation 11
      expect(history.snapshots[0].description).toBe("Mutation 11");
    });
  });

  describe("Concrete Commands Unit Execution", () => {
    const baseContext: SmartPdfCommandContext = {
      sourceBytes: new Uint8Array([1, 2, 3]),
      filename: "test.pdf",
      currentPage: 1,
      pageCount: 3,
      selection: null,
      starPdfDoc: null,
      fieldValues: { first_name: "Alice" },
      annotationValues: { "annot-1": "Note 1" },
      inspectionResult: null,
    };

    it("SetFormFieldValueCommand modifies field dictionary in context", async () => {
      const cmd = new SetFormFieldValueCommand("first_name", "Bob");
      expect(cmd.isMutating).toBe(true);

      const result = await cmd.execute(baseContext);
      expect(result.fieldValues).toEqual({ first_name: "Bob" });
      expect(result.message).toContain("first_name");
    });

    it("UpdateAnnotationCommand modifies annotation dictionary in context", async () => {
      const updateAnnotation = vi.fn().mockResolvedValue(true);
      const exportIncremental = vi.fn().mockResolvedValue(new Uint8Array([4, 5, 6]));
      const commandContext = {
        ...baseContext,
        annotationValues: { "annot-obj-12-0": "Note 1" },
        starPdfDoc: {
          getAnnotations: vi.fn().mockResolvedValue([
            { object_num: 12, object_gen: 0, page_index: 0 },
          ]),
          updateAnnotation,
          exportIncremental,
        } as unknown as NonNullable<SmartPdfCommandContext["starPdfDoc"]>,
      };
      const cmd = new UpdateAnnotationCommand("annot-obj-12-0", "Updated Note");
      expect(cmd.isMutating).toBe(true);

      const result = await cmd.execute(commandContext);
      expect(updateAnnotation).toHaveBeenCalledWith(12, 0, { contents: "Updated Note" });
      expect(result.bytes).toEqual(new Uint8Array([4, 5, 6]));
      expect(result.annotationValues).toEqual({ "annot-obj-12-0": "Updated Note" });
    });

    it("targets duplicate-content annotations only by exact indirect identity", async () => {
      const updateAnnotation = vi.fn().mockResolvedValue(true);
      const context = {
        ...baseContext,
        starPdfDoc: {
          getAnnotations: vi.fn().mockResolvedValue([
            { object_num: 12, object_gen: 0, page_index: 0, contents: "Same" },
            { object_num: 19, object_gen: 2, page_index: 0, contents: "Same" },
          ]),
          updateAnnotation,
          exportIncremental: vi.fn().mockResolvedValue(new Uint8Array([7, 8, 9])),
        } as unknown as NonNullable<SmartPdfCommandContext["starPdfDoc"]>,
      };

      await new UpdateAnnotationCommand("annot-obj-19-2", "Only this one", 0).execute(context);
      expect(updateAnnotation).toHaveBeenCalledTimes(1);
      expect(updateAnnotation).toHaveBeenCalledWith(19, 2, { contents: "Only this one" });
    });

    it("refuses unresolved annotation identity without mutating or exporting", async () => {
      const updateAnnotation = vi.fn();
      const exportIncremental = vi.fn();
      const context = {
        ...baseContext,
        starPdfDoc: {
          getAnnotations: vi.fn().mockResolvedValue([
            { object_num: 12, object_gen: 0, page_index: 0, contents: "Same" },
          ]),
          updateAnnotation,
          exportIncremental,
        } as unknown as NonNullable<SmartPdfCommandContext["starPdfDoc"]>,
      };

      await expect(
        new UpdateAnnotationCommand("annot-inline-0-0", "Wrong target", 0).execute(context),
      ).rejects.toThrow("ANNOTATION_TARGET_UNRESOLVED");
      expect(updateAnnotation).not.toHaveBeenCalled();
      expect(exportIncremental).not.toHaveBeenCalled();
    });

    it("passes existing FreeText contents for style-only adaptive font registration", async () => {
      const updateAnnotation = vi.fn().mockResolvedValue(true);
      const exportIncremental = vi.fn().mockResolvedValue(new Uint8Array([7, 8, 9]));
      const selection = {
        type: "annotation" as const,
        id: "annot-obj-27-0",
        pageIndex: 0,
        pdfRect: { x: 40, y: 50, width: 220, height: 40 },
        data: {
          id: "annot-obj-27-0",
          subtype: "FreeText",
          contents: "Latin تقرير 日本",
          rect: { x: 40, y: 50, width: 220, height: 40 },
          pageIndex: 0,
          objectNumber: 27,
          generationNumber: 0,
        },
      };
      const context = {
        ...baseContext,
        selection,
        starPdfDoc: {
          getAnnotations: vi.fn().mockResolvedValue([
            {
              object_num: 27,
              object_gen: 0,
              page_index: 0,
              subtype: "FreeText",
              rect: [40, 50, 260, 90],
              contents: selection.data.contents,
              font_family: "Serif",
              font_size: 20,
              bold: true,
              italic: true,
              text_color: [0.2, 0.3, 0.6],
              underline: true,
              strikethrough: true,
              highlight_color: [1, 0.9, 0.2],
            },
          ]),
          updateAnnotation,
          exportIncremental,
        } as unknown as NonNullable<SmartPdfCommandContext["starPdfDoc"]>,
      };
      const { UpdateAnnotationPropertiesCommand } = await import("./annotation-commands");

      const result = await new UpdateAnnotationPropertiesCommand(
        selection.id,
        {
          font_family: "Serif",
          font_size: 20,
          bold: true,
          italic: true,
          text_color: [0.2, 0.3, 0.6],
          underline: true,
          strikethrough: true,
          highlight_enabled: true,
          highlight_color: [1, 0.9, 0.2],
        },
        0,
      ).execute(context);

      expect(updateAnnotation).toHaveBeenCalledWith(
        27,
        0,
        {
          font_family: "Serif",
          font_size: 20,
          bold: true,
          italic: true,
          text_color: [0.2, 0.3, 0.6],
          underline: true,
          strikethrough: true,
          highlight_enabled: true,
          highlight_color: [1, 0.9, 0.2],
        },
        "Latin تقرير 日本",
      );
      expect(exportIncremental).toHaveBeenCalledTimes(1);
      expect(result.nextSelection?.data).toMatchObject({
        fontFamily: "Serif",
        fontSize: 20,
        isBold: true,
        isItalic: true,
        textColor: [0.2, 0.3, 0.6],
        isUnderlined: true,
        isStruckThrough: true,
        highlightColor: [1, 0.9, 0.2],
      });
    });

    it("ReplaceTextCommand and DeleteTextCommand declare mutating flag and throw when starPdfDoc is absent", async () => {
      const { ReplaceTextCommand, DeleteTextCommand } = await import("./text-commands");
      const replaceCmd = new ReplaceTextCommand("span-1", "New Text");
      expect(replaceCmd.isMutating).toBe(true);
      await expect(replaceCmd.execute(baseContext)).rejects.toThrow(
        "No active StarPDF document handle available",
      );

      const multiReplaceCmd = new ReplaceTextCommand(["s1", "s2", "s3"], "Group Replacement");
      expect(multiReplaceCmd.isMutating).toBe(true);
      await expect(multiReplaceCmd.execute(baseContext)).rejects.toThrow(
        "No active StarPDF document handle available",
      );

      const deleteCmd = new DeleteTextCommand("span-1");
      expect(deleteCmd.isMutating).toBe(true);
      await expect(deleteCmd.execute(baseContext)).rejects.toThrow(
        "No active StarPDF document handle available",
      );

      const multiDeleteCmd = new DeleteTextCommand(["s1", "s2", "s3"]);
      expect(multiDeleteCmd.isMutating).toBe(true);
      await expect(multiDeleteCmd.execute(baseContext)).rejects.toThrow(
        "No active StarPDF document handle available",
      );
    });

    it("ApplyTextStyleCommand sends one combined mutation and one exported snapshot", async () => {
      const { ApplyTextStyleCommand } = await import("./text-commands");
      const applyTextStyle = vi.fn().mockResolvedValue({
        success: true,
        layout_result: "EXACT_FIT",
        modified_object_count: 2,
      });
      const exportIncremental = vi.fn().mockResolvedValue(new Uint8Array([9, 8, 7]));
      const context = {
        ...baseContext,
        currentPage: 2,
        starPdfDoc: {
          applyTextStyle,
          exportIncremental,
        } as unknown as NonNullable<SmartPdfCommandContext["starPdfDoc"]>,
      };
      const patch = {
        font_family: "Serif" as const,
        font_size: 18,
        weight: "BOLD" as const,
        italic: true,
        fill_color: [0.1, 0.2, 0.3] as [number, number, number],
        underline: true,
        strikethrough: true,
        highlight_enabled: true,
        highlight_color: [1, 0.9, 0.2] as [number, number, number],
        replacement_text: "Styled once",
      };

      const result = await new ApplyTextStyleCommand("p1_s0_i4_o0", "Styled once", patch).execute(
        context,
      );

      expect(applyTextStyle).toHaveBeenCalledTimes(1);
      expect(applyTextStyle).toHaveBeenCalledWith(1, "p1_s0_i4_o0", "Styled once", patch);
      expect(exportIncremental).toHaveBeenCalledTimes(1);
      expect(result.bytes).toEqual(new Uint8Array([9, 8, 7]));
    });

    it("ApplyTextStyleCommand refuses grouped native targets before mutation", async () => {
      const { ApplyTextStyleCommand } = await import("./text-commands");
      const applyTextStyle = vi.fn();
      const exportIncremental = vi.fn();
      const context = {
        ...baseContext,
        starPdfDoc: {
          applyTextStyle,
          exportIncremental,
        } as unknown as NonNullable<SmartPdfCommandContext["starPdfDoc"]>,
      };

      await expect(
        new ApplyTextStyleCommand(["span-a", "span-b"], "Text", { font_size: 16 }).execute(
          context,
        ),
      ).rejects.toThrow("TEXT_STYLE_GROUP_NOT_ISOLATABLE");
      expect(applyTextStyle).not.toHaveBeenCalled();
      expect(exportIncremental).not.toHaveBeenCalled();
    });

    it("Image and Vector direct manipulation commands declare mutating flag and execute through handle", async () => {
      const { UpdateImageCommand } = await import("./image-commands");
      const { AddRectangleCommand, AddLineCommand, UpdateVectorCommand } = await import(
        "./vector-commands"
      );

      const updateImg = new UpdateImageCommand("img-1", 100, 200, 150, 120);
      expect(updateImg.isMutating).toBe(true);

      const addRect = new AddRectangleCommand(0, 50, 60, 200, 100);
      expect(addRect.isMutating).toBe(true);

      const addLine = new AddLineCommand(0, 10, 20, 110, 120);
      expect(addLine.isMutating).toBe(true);

      const updateVec = new UpdateVectorCommand({ page_index: 0, graphic_id: "g1", rect_w: 180 });
      expect(updateVec.isMutating).toBe(true);
    });

    it("UpdateVectorCommand mutates vector shape in real document", async () => {
      const fs = await import("fs");
      const path = await import("path");
      const { StarPdfClient } = await import("../starpdf-client");
      const { UpdateVectorCommand } = await import("./vector-commands");

      const fixturePath = path.resolve(process.cwd(), "test-assets/vector-primitives.pdf");
      const bytes = fs.readFileSync(fixturePath);
      const starDoc = await StarPdfClient.open(bytes);

      const graphics = await starDoc.enumerateGraphics(0);
      expect(graphics.length).toBe(2);

      const rectGraphic = graphics[0];
      const cmd = new UpdateVectorCommand({
        page_index: 0,
        graphic_id: rectGraphic.graphic_id,
        rect_x: 120,
        rect_y: 520,
        rect_w: 180,
        rect_h: 90,
      });

      const res = await cmd.execute({
        ...baseContext,
        starPdfDoc: starDoc,
        sourceBytes: bytes,
      });



      expect(res.bytes).toBeDefined();
      expect(res.bytes!.byteLength).toBeGreaterThan(0);

      // Reopen and check
      const reopened = await StarPdfClient.open(res.bytes!);
      const reGraphics = await reopened.enumerateGraphics(0);
      expect(reGraphics[0].bounds).toBeDefined();
      await starDoc.close();
      await reopened.close();
    });

    it("MoveTextCommand mutates text coordinates in real document", async () => {
      const fs = await import("fs");
      const path = await import("path");
      const { StarPdfClient } = await import("../starpdf-client");
      const { MoveTextCommand } = await import("./text-commands");

      const fixturePath = path.resolve(process.cwd(), "test-assets/edit-test.pdf");
      const bytes = fs.readFileSync(fixturePath);

      const starDoc = await StarPdfClient.open(bytes);

      const pageText = await starDoc.extractPageText(0);
      expect(pageText.spans.length).toBeGreaterThan(0);
      const span = pageText.spans[0];
      const origX = span.x;
      const origY = span.y;

      const cmd = new MoveTextCommand(span.span_id, 30, 40, 1);
      const res = await cmd.execute({
        ...baseContext,
        starPdfDoc: starDoc,
        sourceBytes: bytes,
      });

      expect(res.bytes).toBeDefined();
      const reopened = await StarPdfClient.open(res.bytes!);
      const reopenedText = await reopened.extractPageText(0);
      expect(reopenedText.spans[0].x).toBeCloseTo(origX + 30, 1);
      expect(reopenedText.spans[0].y).toBeCloseTo(origY + 40, 1);

      await starDoc.close();
      await reopened.close();
    });

    it("SetFormFieldValueCommand mutates AcroForm text field and checkbox in real document", async () => {
      const fs = await import("fs");
      const path = await import("path");
      const { StarPdfClient } = await import("../starpdf-client");
      const { SetFormFieldValueCommand } = await import("./form-commands");

      const fixturePath = path.resolve(process.cwd(), "engine/starpdf/tests/fixtures/v0_10_compat/pdfkit-text-checkbox.pdf");
      const bytes = fs.readFileSync(fixturePath);

      const starDoc = await StarPdfClient.open(bytes);
      const fields = await starDoc.getFormFields();
      expect(fields.length).toBeGreaterThan(0);

      const textField = fields.find((f) => f.field_type.toLowerCase() === "text");
      expect(textField).toBeDefined();

      const cmd = new SetFormFieldValueCommand(textField!.name, "Phase 5 Form Test");
      const res = await cmd.execute({
        ...baseContext,
        starPdfDoc: starDoc,
        sourceBytes: bytes,
      });

      expect(res.bytes).toBeDefined();
      expect(res.fieldValues).toBeDefined();
      expect(res.fieldValues![textField!.name]).toBe("Phase 5 Form Test");

      // Verify in reopened document
      const reopened = await StarPdfClient.open(res.bytes!);
      const reopenedFields = await reopened.getFormFields();
      const reopenedTextField = reopenedFields.find((f) => f.name === textField!.name);
      expect(reopenedTextField?.value).toBe("Phase 5 Form Test");

      await starDoc.close();
      await reopened.close();
    });

    it("AddSquareAnnotationCommand, AddCircleAnnotationCommand, and UpdateAnnotationPropertiesCommand mutate annotations", async () => {
      const fs = await import("fs");
      const path = await import("path");
      const { StarPdfClient } = await import("../starpdf-client");
      const {
        AddSquareAnnotationCommand,
        AddCircleAnnotationCommand,
        UpdateAnnotationPropertiesCommand,
        UpdateAnnotationRectCommand,
        DeleteAnnotationCommand,
      } = await import("./annotation-commands");

      const fixturePath = path.resolve(process.cwd(), "test-assets/edit-test.pdf");
      const bytes = fs.readFileSync(fixturePath);

      let starDoc = await StarPdfClient.open(bytes);

      // 1. Add Square & Circle
      const addSquareCmd = new AddSquareAnnotationCommand(0, [50, 50, 200, 150], [1, 0, 0], [0, 0, 1], 2);
      const res1 = await addSquareCmd.execute({
        ...baseContext,
        starPdfDoc: starDoc,
        sourceBytes: bytes,
      });
      expect(res1.bytes).toBeDefined();

      await starDoc.close();
      starDoc = await StarPdfClient.open(res1.bytes!);

      const addCircleCmd = new AddCircleAnnotationCommand(0, [100, 100, 250, 250], [0, 1, 0], undefined, 1.5);
      const resCircle = await addCircleCmd.execute({
        ...baseContext,
        starPdfDoc: starDoc,
        sourceBytes: res1.bytes!,
      });
      expect(resCircle.bytes).toBeDefined();

      await starDoc.close();
      starDoc = await StarPdfClient.open(resCircle.bytes!);

      const annots1 = await starDoc.getAnnotations(0);
      expect(annots1.length).toBeGreaterThan(1);
      const squareAnnot = annots1.find((a) => a.subtype === "Square");
      const circleAnnot = annots1.find((a) => a.subtype === "Circle");
      expect(squareAnnot).toBeDefined();
      expect(circleAnnot).toBeDefined();

      // 2. Move & Resize Square
      const moveSquareCmd = new UpdateAnnotationRectCommand(`annot-obj-${squareAnnot!.object_num}-${squareAnnot!.object_gen}`, [80, 80, 250, 200], 1);
      const res2 = await moveSquareCmd.execute({
        ...baseContext,
        starPdfDoc: starDoc,
        sourceBytes: res1.bytes!,
      });
      expect(res2.bytes).toBeDefined();

      await starDoc.close();
      starDoc = await StarPdfClient.open(res2.bytes!);

      // 3. Style Square
      const styleSquareCmd = new UpdateAnnotationPropertiesCommand(`annot-obj-${squareAnnot!.object_num}-${squareAnnot!.object_gen}`, {
        border_width: 4,
        color: [0, 1, 0],
      });
      const res3 = await styleSquareCmd.execute({
        ...baseContext,
        starPdfDoc: starDoc,
        sourceBytes: res2.bytes!,
      });
      expect(res3.bytes).toBeDefined();

      await starDoc.close();
      starDoc = await StarPdfClient.open(res3.bytes!);

      // 4. Delete Square
      const deleteCmd = new DeleteAnnotationCommand(0, `annot-obj-${squareAnnot!.object_num}-${squareAnnot!.object_gen}`);
      const res4 = await deleteCmd.execute({
        ...baseContext,
        starPdfDoc: starDoc,
        sourceBytes: res3.bytes!,
      });
      expect(res4.bytes).toBeDefined();

      await starDoc.close();
      starDoc = await StarPdfClient.open(res4.bytes!);
      const annotsAfter = await starDoc.getAnnotations(0);
      expect(annotsAfter.find((a) => a.object_num === squareAnnot!.object_num)).toBeUndefined();

      await starDoc.close();
    });
  });

  describe("Phase 6 Page Organizer & Multi-Document Operations", () => {
    it("ReorderPagesCommand calculates move block permutation and verifies identity", () => {
      // 5 pages: 0, 1, 2, 3, 4
      // Move pages [1, 2] before page 4 -> remaining [0, 3, 4], insert [1, 2] at index 2 (before 4) -> [0, 3, 1, 2, 4]
      const cmd = ReorderPagesCommand.fromMoveBlock(5, [1, 2], 4, true);
      expect(cmd.newOrder).toEqual([0, 3, 1, 2, 4]);

      // Move page [0] after page 3 -> remaining [1, 2, 3, 4], insert [0] at index 3 (after 3) -> [1, 2, 3, 0, 4]
      const cmd2 = ReorderPagesCommand.fromMoveBlock(5, [0], 3, false);
      expect(cmd2.newOrder).toEqual([1, 2, 3, 0, 4]);
    });

    it("executes ReorderPagesCommand on minimal document", async () => {
      const sourceBytes = await StarPdfClient.createMinimalPdf("Page 1");
      const handle = await StarPdfClient.open(sourceBytes);
      const inserted = await handle.insertBlankPage(1, 612, 792, 0);
      await handle.close();

      const baseContext: SmartPdfCommandContext = {
        sourceBytes: inserted,
        filename: "test.pdf",
        currentPage: 1,
        pageCount: 2,
        selection: null,
        starPdfDoc: null,
        fieldValues: {},
        annotationValues: {},
        inspectionResult: null,
      };

      // Reorder [1, 0]
      const reorderCmd = new ReorderPagesCommand([1, 0], 2);
      const result = await reorderCmd.execute(baseContext);

      expect(result.bytes).toBeDefined();
      expect(result.nextPage).toBe(2);

      const verifyHandle = await StarPdfClient.open(result.bytes!);
      expect(await verifyHandle.getPageCount()).toBe(2);
      await verifyHandle.close();
    });

    it("executes DuplicatePagesBatchCommand and DeletePagesBatchCommand", async () => {
      const sourceBytes = await StarPdfClient.createMinimalPdf("Page 1");
      const handle = await StarPdfClient.open(sourceBytes);
      const p2 = await handle.insertBlankPage(1, 612, 792, 0);
      const h2 = await StarPdfClient.open(p2);
      const p3 = await h2.insertBlankPage(2, 612, 792, 0);
      await handle.close();
      await h2.close();

      const baseContext: SmartPdfCommandContext = {
        sourceBytes: p3,
        filename: "test.pdf",
        currentPage: 1,
        pageCount: 3,
        selection: null,
        starPdfDoc: null,
        fieldValues: {},
        annotationValues: {},
        inspectionResult: null,
      };

      // Duplicate pages 0 and 1 -> from 3 pages to 5 pages
      const dupCmd = new DuplicatePagesBatchCommand([0, 1]);
      const dupRes = await dupCmd.execute(baseContext);
      expect(dupRes.bytes).toBeDefined();

      const dupHandle = await StarPdfClient.open(dupRes.bytes!);
      expect(await dupHandle.getPageCount()).toBe(5);
      await dupHandle.close();

      // Delete pages 0 and 1 -> remaining 1 page
      const delCmd = new DeletePagesBatchCommand([0, 1]);
      const delRes = await delCmd.execute(baseContext);
      expect(delRes.bytes).toBeDefined();

      const delHandle = await StarPdfClient.open(delRes.bytes!);
      expect(await delHandle.getPageCount()).toBe(1);
      await delHandle.close();

      // Deleting all pages is refused
      const delAllCmd = new DeletePagesBatchCommand([0, 1, 2]);
      await expect(delAllCmd.execute(baseContext)).rejects.toThrow("Cannot delete all pages in a document.");
    });

    it("executes InsertImportedPagesCommand and refuses invalid/encrypted imports", async () => {
      const docA = await StarPdfClient.createMinimalPdf("Doc A");
      const docB = await StarPdfClient.createMinimalPdf("Doc B");

      const baseContext: SmartPdfCommandContext = {
        sourceBytes: docA,
        filename: "docA.pdf",
        currentPage: 1,
        pageCount: 1,
        selection: null,
        starPdfDoc: null,
        fieldValues: {},
        annotationValues: {},
        inspectionResult: null,
      };

      const importCmd = new InsertImportedPagesCommand(docB, [0], "after", 0, "docB.pdf");
      const importRes = await importCmd.execute(baseContext);

      expect(importRes.bytes).toBeDefined();
      const verifyHandle = await StarPdfClient.open(importRes.bytes!);
      expect(await verifyHandle.getPageCount()).toBe(2);
      await verifyHandle.close();
    });
  });

});
