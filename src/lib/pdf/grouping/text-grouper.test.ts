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

  it("groups contiguous sub-words into a single human text run while retaining all source spans", () => {
    const span1 = createSpan({ text: "P", x: 50, y: 700, width: 8, span_id: "s1" });
    const span2 = createSpan({ text: "atient", x: 58, y: 700, width: 30, span_id: "s2" });
    const span3 = createSpan({ text: "Name", x: 92, y: 700, width: 28, span_id: "s3" });

    const groups = groupTextSpans([span1, span2, span3]);

    expect(groups).toHaveLength(1);
    expect(groups[0].text).toBe("Patient Name");
    expect(groups[0].sourceSpans).toHaveLength(3);
    expect(groups[0].editability).toBe("GROUP_SELECTION_ONLY");
    expect(groups[0].is_editable).toBe(false);
    expect(groups[0].refusal_reason).toContain("multiple PDF text operations");
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
    const line1 = createSpan({ text: "First Line of Paragraph", x: 50, y: 700, width: 150 });
    const line2 = createSpan({ text: "Second Line of Paragraph", x: 50, y: 680, width: 160 }); // y diff = 20 > 2.0

    const groups = groupTextSpans([line1, line2]);

    expect(groups).toHaveLength(2);
    expect(groups[0].text).toBe("First Line of Paragraph");
    expect(groups[1].text).toBe("Second Line of Paragraph");
  });

  it("does NOT group across different rotation angles", () => {
    const horizontal = createSpan({ text: "Horizontal Text", x: 50, y: 700, rotation: 0 });
    const rotated = createSpan({ text: "Vertical Stamp", x: 50, y: 700, rotation: 90 });

    const groups = groupTextSpans([horizontal, rotated]);

    expect(groups).toHaveLength(2);
  });
});
