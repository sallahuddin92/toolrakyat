import type { StarPdfTextSpan } from "../starpdf-types";

export type HumanGroupType = "word" | "run" | "line";

export type GroupEditability =
  | "EDITABLE_ATOMIC"
  | "GROUP_SELECTION_ONLY"
  | "READ_ONLY_REFUSAL";

export interface HumanTextSpanRef {
  span_id: string;
  stream_index: number;
  instruction_index: number;
  operand_index: number;
  text: string;
  is_editable: boolean;
  refusal_reason?: string;
}

export interface HumanTextGroup {
  id: string;
  pageIndex: number;
  text: string;
  type: HumanGroupType;
  x: number;
  y: number;
  width: number;
  height: number;
  rotation: number;
  fontSize: number;
  fontName: string;
  is_editable: boolean;
  editability: GroupEditability;
  refusal_reason?: string;
  detailed_reason?: string;
  sourceSpans: StarPdfTextSpan[];
  primarySpanId: string;
}

export interface TextGroupingOptions {
  /** Maximum baseline distance difference in points to consider spans on the same line (default: 2.0) */
  maxBaselineDiff?: number;
  /** Maximum horizontal gap multiplier of space width before breaking group (default: 1.8) */
  maxHorizontalGapMultiplier?: number;
  /** Maximum font size difference in points (default: 1.5) */
  maxFontSizeDiff?: number;
}
