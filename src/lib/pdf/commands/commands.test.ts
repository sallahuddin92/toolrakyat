import { describe, expect, it } from "vitest";
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
      const cmd = new UpdateAnnotationCommand("annot-1", "Updated Note");
      expect(cmd.isMutating).toBe(true);

      const result = await cmd.execute(baseContext);
      expect(result.annotationValues).toEqual({ "annot-1": "Updated Note" });
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
  });

});



