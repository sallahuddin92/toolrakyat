import { describe, it, expect } from "vitest";
import { groupTextSpans } from "./text-grouper";
import type { StarPdfTextSpan } from "../starpdf-types";

function createSpan(partial: Partial<StarPdfTextSpan>): StarPdfTextSpan {
  return {
    page_index: 0,
    text: "Sample",
    x: 50,
    y: 700,
    width: 40,
    height: 12,
    rotation: 0,
    font_name: "F1",
    font_size: 12,
    confidence: 1.0,
    span_id: `span-${Math.random().toString(36).slice(2, 8)}`,
    stream_index: 0,
    instruction_index: 0,
    operand_index: 0,
    operator_name: "Tj",
    is_editable: true,
    editability_code: "EDITABLE_NATIVE_TEXT",
    ...partial,
  };
}

describe("groupTextSpans", () => {
  it("returns empty array for empty spans", () => {
    expect(groupTextSpans([])).toEqual([]);
  });

  it("handles single editable span as EDITABLE_ATOMIC", () => {
    const span = createSpan({ text: "Invoice #1234", is_editable: true, span_id: "s1" });
    const groups = groupTextSpans([span]);

    expect(groups).toHaveLength(1);
    expect(groups[0].text).toBe("Invoice #1234");
    expect(groups[0].is_editable).toBe(true);
    expect(groups[0].editability).toBe("EDITABLE_ATOMIC");
    expect(groups[0].sourceSpans).toHaveLength(1);
    expect(groups[0].primarySpanId).toBe("s1");
  });

  it("handles single uneditable span as READ_ONLY_REFUSAL with reason", () => {
    const span = createSpan({
      text: "Specialized Encoded Text",
      is_editable: false,
      refusal_reason: "Specialized font encoding",
    });
    const groups = groupTextSpans([span]);

    expect(groups).toHaveLength(1);
    expect(groups[0].is_editable).toBe(false);
    expect(groups[0].editability).toBe("READ_ONLY_REFUSAL");
    expect(groups[0].refusal_reason).toBe("Specialized font encoding");
  });

  it("groups contiguous intra-word fragments into an EDITABLE_ATOMIC word group", () => {
    const span1 = createSpan({ text: "Arch", x: 50, y: 700, width: 25, span_id: "s1", font_name: "F1", is_editable: true });
    const span2 = createSpan({ text: "itect", x: 75, y: 700, width: 25, span_id: "s2", font_name: "F1", is_editable: true });
    const span3 = createSpan({ text: "ural", x: 100, y: 700, width: 20, span_id: "s3", font_name: "F1", is_editable: true });

    const groups = groupTextSpans([span1, span2, span3]);

    expect(groups).toHaveLength(1);
    expect(groups[0].text).toBe("Architectural");
    expect(groups[0].sourceSpans).toHaveLength(3);
    expect(groups[0].editability).toBe("EDITABLE_ATOMIC");
    expect(groups[0].is_editable).toBe(true);
    expect(groups[0].type).toBe("word");
  });

  it("separates words across spaces into independent word groups", () => {
    const span1 = createSpan({ text: "Star", x: 50, y: 700, width: 22, span_id: "s1", font_name: "F1" });
    const span2 = createSpan({ text: "Orion", x: 72, y: 700, width: 30, span_id: "s2", font_name: "F1" });
    // "Renaissance" starts after a normal inter-word space (gap of 8pt > spaceWidth * 0.45)
    const span3 = createSpan({ text: "Renaissance", x: 110, y: 700, width: 65, span_id: "s3", font_name: "F1" });

    const groups = groupTextSpans([span1, span2, span3]);

    expect(groups).toHaveLength(2);
    expect(groups[0].text).toBe("StarOrion");
    expect(groups[0].sourceSpans).toHaveLength(2);
    expect(groups[0].editability).toBe("EDITABLE_ATOMIC");
    expect(groups[1].text).toBe("Renaissance");
    expect(groups[1].sourceSpans).toHaveLength(1);
    expect(groups[1].editability).toBe("EDITABLE_ATOMIC");
  });

  it("safely marks mixed-font multi-span words as READ_ONLY_REFUSAL", () => {
    const span1 = createSpan({ text: "Arch", x: 50, y: 700, width: 25, span_id: "s1", font_name: "F1", is_editable: true });
    const span2 = createSpan({ text: "itectural", x: 75, y: 700, width: 45, span_id: "s2", font_name: "F2", is_editable: true });

    const groups = groupTextSpans([span1, span2]);

    expect(groups).toHaveLength(2);
    expect(groups[0].text).toBe("Arch");
    expect(groups[1].text).toBe("itectural");
  });

  it("does NOT group across multi-column gap", () => {
    // Column 1 at x=50, width=150
    const col1 = createSpan({ text: "Left Column Article Text", x: 50, y: 700, width: 150, span_id: "col1" });
    // Column 2 at x=300 (gap of 100pt > maxAllowedGap)
    const col2 = createSpan({ text: "Right Column Article Text", x: 300, y: 700, width: 150, span_id: "col2" });

    const groups = groupTextSpans([col1, col2]);

    expect(groups).toHaveLength(2);
    expect(groups[0].text).toBe("Left Column Article Text");
    expect(groups[1].text).toBe("Right Column Article Text");
  });

  it("does NOT group across table cells on the same row", () => {
    const cellA = createSpan({ text: "Item 001", x: 50, y: 500, width: 60, span_id: "cA" });
    const cellB = createSpan({ text: "Medical Supplies", x: 150, y: 500, width: 100, span_id: "cB" });
    const cellC = createSpan({ text: "RM 250.00", x: 400, y: 500, width: 60, span_id: "cC" });

    const groups = groupTextSpans([cellA, cellB, cellC]);

    expect(groups).toHaveLength(3);
    expect(groups[0].text).toBe("Item 001");
    expect(groups[1].text).toBe("Medical Supplies");
    expect(groups[2].text).toBe("RM 250.00");
  });

  it("does NOT group across different lines / baselines", () => {
    const line1 = createSpan({ text: "Line One Heading", x: 50, y: 700, width: 100, span_id: "l1" });
    const line2 = createSpan({ text: "Line Two Subtitle", x: 50, y: 680, width: 100, span_id: "l2" });

    const groups = groupTextSpans([line1, line2]);

    expect(groups).toHaveLength(2);
    expect(groups[0].text).toBe("Line One Heading");
    expect(groups[1].text).toBe("Line Two Subtitle");
  });

  it("does NOT group across different rotation angles", () => {
    const horizontal = createSpan({ text: "Horizontal Text", x: 50, y: 700, rotation: 0 });
    const rotated = createSpan({ text: "Vertical Stamp", x: 50, y: 700, rotation: 90 });

    const groups = groupTextSpans([horizontal, rotated]);

    expect(groups).toHaveLength(2);
  });
});
