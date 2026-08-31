"use client";

import { useState, useRef, useEffect, useCallback } from "react";

import { Button } from "@/components/ui/button";

import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Type,
  ImageIcon,
  Shapes,
  FileEdit,
  X,
  Check,
  Upload,
  Trash2,
  Lock,
  Bold as BoldIcon,
  Italic as ItalicIcon,
  Underline as UnderlineIcon,
  Strikethrough as StrikethroughIcon,
  Highlighter,
} from "lucide-react";
import {
  STARPDF_TEXT_FONT_SIZE_MAX,
  STARPDF_TEXT_FONT_SIZE_MIN,
} from "@/lib/pdf/starpdf-types";
import type {
  StarPdfTextSpan,
  StarPdfImageInfo,
  StarPdfVectorGraphicInfo,
  StarPdfUpdateVectorGraphicInput,
  StarPdfUpdateAnnotationInput,
  StarPdfTextStylePatch,
} from "@/lib/pdf/starpdf-types";
import type { AcroFormField, PdfMarkupAnnotation } from "@/lib/pdf/pdf-types";
import type { SmartPdfSelection, SelectionType } from "@/lib/pdf/selection";

export type { SelectionType };
export type SelectedItem = NonNullable<SmartPdfSelection>;

interface PdfContextualToolbarProps {
  selection: SmartPdfSelection;
  containerRef?: React.RefObject<HTMLElement | null>;
  onDeselect: () => void;
  onReplaceText: (spanId: string | string[], newText: string) => Promise<void>;
  onApplyTextStyle: (
    spanId: string | string[],
    text: string,
    patch: StarPdfTextStylePatch,
  ) => Promise<void>;
  onDeleteText?: (spanId: string | string[]) => Promise<void>;
  onReplaceImage: (imageId: string, file: File) => Promise<void>;
  onRemoveImage: (imageId: string) => Promise<void>;
  onUpdateGraphic: (input: StarPdfUpdateVectorGraphicInput) => Promise<void>;
  onDeleteGraphic: (graphicId: string) => Promise<void>;
  onFormFieldChange: (fieldName: string, value: string | boolean | string[]) => void;
  formFieldValue?: string | boolean | string[];
  onAnnotationChange?: (annotId: string, value: string) => Promise<void>;
  annotationValue?: string;
  onUpdateAnnotationProperties?: (annotId: string, properties: StarPdfUpdateAnnotationInput) => Promise<void>;
  onDeleteAnnotation?: (annotId: string) => Promise<void>;
  onAddTextInstead?: () => void;
}

const hexToRgbTuple = (hex: string): [number, number, number] => {
  const clean = hex.replace("#", "");
  return [
    Number.parseInt(clean.slice(0, 2), 16) / 255,
    Number.parseInt(clean.slice(2, 4), 16) / 255,
    Number.parseInt(clean.slice(4, 6), 16) / 255,
  ];
};


function TextControls({
  span,
  group,
  onReplaceText,
  onApplyTextStyle,
  onDeleteText,
  onDeselect,
  onAddTextInstead,
}: {
  span: StarPdfTextSpan;
  group?: import("@/lib/pdf/grouping").HumanTextGroup;
  onReplaceText: (spanId: string | string[], newText: string) => Promise<void>;
  onApplyTextStyle: (
    spanId: string | string[],
    text: string,
    patch: StarPdfTextStylePatch,
  ) => Promise<void>;
  onDeleteText?: (spanId: string | string[]) => Promise<void>;
  onDeselect: () => void;
  onAddTextInstead?: () => void;
}) {
  const initialTextColor = `#${(span.fill_color ?? [0, 0, 0])
    .map((channel) => Math.round(channel * 255).toString(16).padStart(2, "0"))
    .join("")}`;
  const originalText = group?.text || span.text;
  const [editText, setEditText] = useState(originalText);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showDetails, setShowDetails] = useState(false);
  const initialFamily =
    span.font_family === "Serif" || span.font_family === "Monospace"
      ? span.font_family
      : "SansSerif";
  const [fontFamily, setFontFamily] = useState<"SansSerif" | "Serif" | "Monospace">(
    initialFamily,
  );
  const [fontSize, setFontSize] = useState(span.font_size);
  const [isBold, setIsBold] = useState(Boolean(span.is_bold));
  const [isItalic, setIsItalic] = useState(Boolean(span.is_italic));
  const [textColor, setTextColor] = useState(initialTextColor);
  const [isUnderlined, setIsUnderlined] = useState(Boolean(span.underline));
  const [isStruckThrough, setIsStruckThrough] = useState(Boolean(span.strikethrough));
  const initialHighlightColor = span.highlight_color
    ? `#${span.highlight_color.map((channel) => Math.round(channel * 255).toString(16).padStart(2, "0")).join("")}`
    : "#ffeb3b";
  const [isHighlighted, setIsHighlighted] = useState(Boolean(span.highlight_color));
  const [nativeHighlightColor, setNativeHighlightColor] = useState(initialHighlightColor);

  const isEditable = group ? group.editability === "EDITABLE_ATOMIC" : span.is_editable;
  const targetSpanIds =
    group && group.sourceSpans.length > 0
      ? group.sourceSpans.map((s) => s.span_id)
      : span.span_id;
  const formattingAvailable =
    !Array.isArray(targetSpanIds) || targetSpanIds.length === 1;

  if (!isEditable) {
    const detailedReason =
      group?.detailed_reason ||
      span.refusal_reason ||
      "This text uses an encoding or layout structure that StarPDF cannot safely rewrite without risking distortion.";

    return (
      <div className="flex items-center gap-2 text-xs text-slate-600 flex-wrap" data-testid="context-text-controls">
        <Badge variant="secondary" className="bg-slate-100 text-slate-700 gap-1 font-normal">
          <Lock className="size-3 text-slate-400" /> Read-Only
        </Badge>
        <span className="font-medium text-slate-700" data-testid="context-text-refusal-msg">
          This text can&apos;t be safely edited in place.
        </span>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => setShowDetails((prev) => !prev)}
          className="h-6 px-1.5 text-[11px] text-slate-500 hover:text-slate-800 underline"
          data-testid="context-text-details-btn"
        >
          {showDetails ? "Hide details" : "Details"}
        </Button>
        {onAddTextInstead && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onAddTextInstead}
            className="h-6 px-2 text-[11px] bg-white border-slate-300 text-slate-700 hover:bg-slate-50"
            data-testid="context-text-add-instead-btn"
          >
            Add new text instead
          </Button>
        )}
        {showDetails && (
          <div
            className="w-full mt-1 p-2 rounded bg-slate-50 border border-slate-200 text-[11px] text-slate-600 leading-relaxed"
            data-testid="context-text-details-panel"
          >
            {detailedReason}
          </div>
        )}
      </div>
    );
  }

  const handleTextSubmit = async () => {
    const styleChanged =
      fontFamily !== initialFamily ||
      Math.abs(fontSize - span.font_size) > 0.001 ||
      isBold !== Boolean(span.is_bold) ||
      isItalic !== Boolean(span.is_italic) ||
      textColor !== initialTextColor;
    const decorationChanged =
      isUnderlined !== Boolean(span.underline) ||
      isStruckThrough !== Boolean(span.strikethrough) ||
      isHighlighted !== Boolean(span.highlight_color) ||
      (isHighlighted && nativeHighlightColor !== initialHighlightColor);
    if (editText === originalText && !styleChanged && !decorationChanged) {
      onDeselect();
      return;
    }
    setIsSubmitting(true);
    try {
      if ((styleChanged || decorationChanged) && editText.trim()) {
        const channels = textColor
          .replace("#", "")
          .match(/.{2}/g)
          ?.map((part) => Number.parseInt(part, 16) / 255);
        await onApplyTextStyle(targetSpanIds, editText, {
          ...(fontFamily !== initialFamily ? { font_family: fontFamily } : {}),
          ...(Math.abs(fontSize - span.font_size) > 0.001 ? { font_size: fontSize } : {}),
          ...(isBold !== Boolean(span.is_bold)
            ? { weight: isBold ? "BOLD" : "NORMAL" }
            : {}),
          ...(isItalic !== Boolean(span.is_italic) ? { italic: isItalic } : {}),
          ...(textColor !== initialTextColor && channels?.length === 3
            ? { fill_color: channels as [number, number, number] }
            : {}),
          ...(isUnderlined !== Boolean(span.underline) ? { underline: isUnderlined } : {}),
          ...(isStruckThrough !== Boolean(span.strikethrough)
            ? { strikethrough: isStruckThrough }
            : {}),
          ...(isHighlighted !== Boolean(span.highlight_color)
            ? { highlight_enabled: isHighlighted }
            : {}),
          ...(isHighlighted && nativeHighlightColor !== initialHighlightColor
            ? { highlight_color: hexToRgbTuple(nativeHighlightColor) }
            : {}),
          ...(editText !== originalText ? { replacement_text: editText } : {}),
        });
      } else if (!editText.trim()) {
        if (onDeleteText) {
          await onDeleteText(targetSpanIds);
        } else {
          await onReplaceText(targetSpanIds, "");
        }
      } else {
        await onReplaceText(targetSpanIds, editText);
      }
    } finally {
      setIsSubmitting(false);
    }
  };


  return (
    <div className="flex items-center gap-1.5 min-w-0 flex-wrap" data-testid="context-text-controls">
      <Input
        type="text"
        value={editText}
        onChange={(e) => setEditText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void handleTextSubmit();
          if (e.key === "Escape") {
            setEditText(originalText);
            setFontFamily(initialFamily);
            setFontSize(span.font_size);
            setIsBold(Boolean(span.is_bold));
            setIsItalic(Boolean(span.is_italic));
            setTextColor(initialTextColor);
            setIsUnderlined(Boolean(span.underline));
            setIsStruckThrough(Boolean(span.strikethrough));
            setIsHighlighted(Boolean(span.highlight_color));
            setNativeHighlightColor(initialHighlightColor);
            onDeselect();
          }
        }}
        className="h-8 text-xs w-36 sm:w-52 bg-white"
        placeholder="Edit text content..."
        data-testid="context-text-input"
        autoFocus
      />
      {formattingAvailable ? (
        <>
          <select
            value={fontFamily}
            onChange={(event) => setFontFamily(event.target.value as typeof fontFamily)}
            className="h-8 rounded border border-slate-300 bg-white px-1.5 text-xs"
            aria-label="Font family"
            data-testid="context-text-font-family"
          >
            <option value={initialFamily}>Current ({span.font_base_name || span.font_name})</option>
            {initialFamily !== "SansSerif" && <option value="SansSerif">Helvetica</option>}
            {initialFamily !== "Serif" && <option value="Serif">Times</option>}
            {initialFamily !== "Monospace" && <option value="Monospace">Courier</option>}
          </select>
          <Input
            type="number"
            min={STARPDF_TEXT_FONT_SIZE_MIN}
            max={STARPDF_TEXT_FONT_SIZE_MAX}
            step={0.5}
            value={fontSize}
            onChange={(event) => setFontSize(Number(event.target.value))}
            className="h-8 w-16 px-1.5 text-xs"
            aria-label="Font size"
            data-testid="context-text-font-size"
          />
          <Button
            type="button"
            variant={isBold ? "default" : "outline"}
            size="icon"
            onClick={() => setIsBold((value) => !value)}
            className="size-8"
            aria-label="Bold"
            aria-pressed={isBold}
            data-testid="context-text-bold"
          >
            <BoldIcon className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant={isItalic ? "default" : "outline"}
            size="icon"
            onClick={() => setIsItalic((value) => !value)}
            className="size-8"
            aria-label="Italic"
            aria-pressed={isItalic}
            data-testid="context-text-italic"
          >
            <ItalicIcon className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant={isUnderlined ? "default" : "outline"}
            size="icon"
            onClick={() => setIsUnderlined((value) => !value)}
            className="size-8"
            aria-label="Underline"
            aria-pressed={isUnderlined}
            data-testid="context-text-underline"
          >
            <UnderlineIcon className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant={isStruckThrough ? "default" : "outline"}
            size="icon"
            onClick={() => setIsStruckThrough((value) => !value)}
            className="size-8"
            aria-label="Strikethrough"
            aria-pressed={isStruckThrough}
            data-testid="context-text-strikethrough"
          >
            <StrikethroughIcon className="size-3.5" />
          </Button>
          <input
            type="color"
            value={textColor}
            onChange={(event) => setTextColor(event.target.value)}
            className="size-8 rounded border border-slate-300 bg-white p-0.5"
            aria-label="Text color"
            data-testid="context-text-color"
          />
          <Button
            type="button"
            variant={isHighlighted ? "default" : "outline"}
            size="icon"
            onClick={() => setIsHighlighted((value) => !value)}
            className="size-8"
            aria-label="Highlight"
            aria-pressed={isHighlighted}
            data-testid="context-text-highlight"
          >
            <Highlighter className="size-3.5" />
          </Button>
          {isHighlighted && (
            <input
              type="color"
              value={nativeHighlightColor}
              onChange={(event) => setNativeHighlightColor(event.target.value)}
              className="size-8 rounded border border-slate-300 bg-white p-0.5"
              aria-label="Highlight color"
              data-testid="context-text-highlight-color"
            />
          )}
        </>
      ) : (
        <span
          className="text-[11px] text-slate-600"
          data-testid="context-text-formatting-unavailable"
        >
          Formatting is available for a single text run.
        </span>
      )}
      <Button
        type="button"
        size="sm"
        onClick={() => void handleTextSubmit()}
        disabled={isSubmitting}
        className="h-8 text-xs gap-1 bg-sky-600 hover:bg-sky-700 text-white shrink-0"
        data-testid="context-text-save-btn"
      >
        <Check className="size-3.5" />
        <span>Apply</span>
      </Button>
      {editText && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => setEditText("")}
          className="h-8 text-xs px-2 text-slate-500 hover:text-slate-700 shrink-0"
          title="Clear text input"
          data-testid="context-text-clear-btn"
        >
          Clear
        </Button>
      )}
      {onDeleteText && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => void onDeleteText(group?.primarySpanId || span.span_id)}
          disabled={isSubmitting}
          className="h-8 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50 shrink-0"
          title="Delete native text"
          data-testid="context-text-delete-btn"
        >
          <Trash2 className="size-3.5" />
          <span>Delete</span>
        </Button>
      )}
    </div>
  );
}



function ImageControls({
  img,
  onReplaceImage,
  onRemoveImage,
}: {
  img: StarPdfImageInfo;
  onReplaceImage: (imageId: string, file: File) => Promise<void>;
  onRemoveImage: (imageId: string) => Promise<void>;
}) {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const handleImageFileChange = async (files: FileList | null) => {
    if (!files || files.length === 0) return;
    setIsSubmitting(true);
    try {
      await onReplaceImage(img.image_id, files[0]);
    } finally {
      setIsSubmitting(false);
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  };

  return (
    <div className="flex items-center gap-2">
      <Badge variant="outline" className="text-[11px] font-normal text-slate-600 bg-slate-50">
        {img.filter === "DCTDecode" ? "JPEG" : "Flate"} ({Math.round(img.width)}×{Math.round(img.height)} pt)
      </Badge>
      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={() => fileInputRef.current?.click()}
        disabled={isSubmitting}
        className="h-8 text-xs gap-1 text-slate-700"
        data-testid="context-image-replace-btn"
      >
        <Upload className="size-3.5" />
        <span>Replace</span>
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={() => void onRemoveImage(img.image_id)}
        disabled={isSubmitting}
        className="h-8 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50"
        data-testid="context-image-remove-btn"
      >
        <Trash2 className="size-3.5" />
        <span>Remove</span>
      </Button>
      <input
        ref={fileInputRef}
        type="file"
        accept="image/png,image/jpeg,image/jpg"
        className="hidden"
        onChange={(e) => void handleImageFileChange(e.target.files)}
      />
    </div>
  );
}

function VectorControls({
  graphic,
  onUpdateGraphic,
  onDeleteGraphic,
}: {
  graphic: StarPdfVectorGraphicInfo;
  onUpdateGraphic: (input: StarPdfUpdateVectorGraphicInput) => Promise<void>;
  onDeleteGraphic: (graphicId: string) => Promise<void>;
}) {
  const [lineWidth, setLineWidth] = useState(graphic.line_width || 1.5);
  const [strokeColor, setStrokeColor] = useState(() => {
    if (graphic.stroke_color_rgb) {
      const [r, gVal, b] = graphic.stroke_color_rgb;
      return (
        "#" +
        [r, gVal, b]
          .map((c) => Math.round(c * 255).toString(16).padStart(2, "0"))
          .join("")
      );
    }
    return "#000000";
  });
  const [fillColor, setFillColor] = useState(() => {
    if (graphic.fill_color_rgb) {
      const [r, gVal, b] = graphic.fill_color_rgb;
      return (
        "#" +
        [r, gVal, b]
          .map((c) => Math.round(c * 255).toString(16).padStart(2, "0"))
          .join("")
      );
    }
    return "#3b82f6";
  });
  const [isSubmitting, setIsSubmitting] = useState(false);

  const hexToRgb = (hex: string): [number, number, number] => {
    const clean = hex.replace("#", "");
    return [
      parseInt(clean.substring(0, 2), 16) / 255,
      parseInt(clean.substring(2, 4), 16) / 255,
      parseInt(clean.substring(4, 6), 16) / 255,
    ];
  };

  const handleVectorColorChange = async (newStroke?: string, newFill?: string, newWidth?: number) => {
    const strokeHex = newStroke ?? strokeColor;
    const fillHex = newFill ?? fillColor;
    const strokeRgb = graphic.is_stroked ? hexToRgb(strokeHex) : undefined;
    const fillRgb = graphic.is_filled ? hexToRgb(fillHex) : undefined;

    setIsSubmitting(true);
    try {
      await onUpdateGraphic({
        page_index: graphic.page_index,
        graphic_id: graphic.graphic_id,
        stroke_color_rgb: strokeRgb,
        fill_color_rgb: fillRgb,
        line_width: newWidth ?? lineWidth,
        is_stroked: graphic.is_stroked,
        is_filled: graphic.is_filled,
        clone_if_shared: true,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="flex items-center gap-2">
      {graphic.is_stroked && (
        <div className="flex items-center gap-1">
          <span className="text-[11px] text-slate-500">Stroke:</span>
          <input
            type="color"
            value={strokeColor}
            onChange={(e) => {
              setStrokeColor(e.target.value);
              void handleVectorColorChange(e.target.value);
            }}
            className="size-6 rounded border border-slate-300 cursor-pointer p-0"
            title="Stroke Color"
            data-testid="context-vector-stroke-color"
          />
        </div>
      )}
      {graphic.is_filled && (
        <div className="flex items-center gap-1">
          <span className="text-[11px] text-slate-500">Fill:</span>
          <input
            type="color"
            value={fillColor}
            onChange={(e) => {
              setFillColor(e.target.value);
              void handleVectorColorChange(undefined, e.target.value);
            }}
            className="size-6 rounded border border-slate-300 cursor-pointer p-0"
            title="Fill Color"
            data-testid="context-vector-fill-color"
          />
        </div>
      )}
      <div className="flex items-center gap-1">
        <span className="text-[11px] text-slate-500">Width:</span>
        <select
          value={lineWidth}
          onChange={(e) => {
            const val = parseFloat(e.target.value);
            setLineWidth(val);
            void handleVectorColorChange(undefined, undefined, val);
          }}
          className="h-7 text-xs rounded border border-slate-300 bg-white px-1.5 text-slate-700"
          data-testid="context-vector-line-width"
        >
          <option value="0.5">0.5 pt</option>
          <option value="1.0">1.0 pt</option>
          <option value="1.5">1.5 pt</option>
          <option value="2.0">2.0 pt</option>
          <option value="3.0">3.0 pt</option>
          <option value="5.0">5.0 pt</option>
        </select>
      </div>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={() => void onDeleteGraphic(graphic.graphic_id)}
        disabled={isSubmitting}
        className="h-8 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50"
        data-testid="context-vector-delete-btn"
      >
        <Trash2 className="size-3.5" />
        <span>Delete</span>
      </Button>
    </div>
  );
}

function AnnotationControls({
  annot,
  annotationValue,
  onAnnotationChange,
  onUpdateAnnotationProperties,
  onDeleteAnnotation,
  onDeselect,
}: {
  annot: PdfMarkupAnnotation;
  annotationValue?: string;
  onAnnotationChange?: (annotId: string, value: string) => Promise<void>;
  onUpdateAnnotationProperties?: (annotId: string, properties: StarPdfUpdateAnnotationInput) => Promise<void>;
  onDeleteAnnotation?: (annotId: string) => Promise<void>;
  onDeselect?: () => void;
}) {
  const sourceText = annotationValue ?? annot.contents ?? "";
  const [localText, setLocalText] = useState(sourceText);
  const [strokeColor, setStrokeColor] = useState("#000000");
  const [fillColor, setFillColor] = useState("#3b82f6");
  const [borderWidth, setBorderWidth] = useState(1.5);
  const [highlightColor, setHighlightColor] = useState("#ffeb3b");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [freeTextFamily, setFreeTextFamily] = useState<"SansSerif" | "Serif" | "Monospace">(
    annot.fontFamily ?? "SansSerif",
  );
  const [freeTextSize, setFreeTextSize] = useState(annot.fontSize ?? 12);
  const [freeTextBold, setFreeTextBold] = useState(Boolean(annot.isBold));
  const [freeTextItalic, setFreeTextItalic] = useState(Boolean(annot.isItalic));
  const initialTextColor = annot.textColor
    ? `#${annot.textColor
        .map((component) => Math.round(component * 255).toString(16).padStart(2, "0"))
        .join("")}`
    : "#000000";
  const [freeTextColor, setFreeTextColor] = useState(initialTextColor);
  const [freeTextUnderlined, setFreeTextUnderlined] = useState(Boolean(annot.isUnderlined));
  const [freeTextStruckThrough, setFreeTextStruckThrough] = useState(
    Boolean(annot.isStruckThrough),
  );
  const initialFreeTextHighlightColor = annot.highlightColor
    ? `#${annot.highlightColor.map((component) => Math.round(component * 255).toString(16).padStart(2, "0")).join("")}`
    : "#ffeb3b";
  const [freeTextHighlighted, setFreeTextHighlighted] = useState(Boolean(annot.highlightColor));
  const [freeTextHighlightColor, setFreeTextHighlightColor] = useState(
    initialFreeTextHighlightColor,
  );

  const freeTextStyleChanged =
    freeTextFamily !== (annot.fontFamily ?? "SansSerif") ||
    Math.abs(freeTextSize - (annot.fontSize ?? 12)) > 0.001 ||
    freeTextBold !== Boolean(annot.isBold) ||
    freeTextItalic !== Boolean(annot.isItalic) ||
    freeTextColor !== initialTextColor ||
    freeTextUnderlined !== Boolean(annot.isUnderlined) ||
    freeTextStruckThrough !== Boolean(annot.isStruckThrough) ||
    freeTextHighlighted !== Boolean(annot.highlightColor) ||
    (freeTextHighlighted && freeTextHighlightColor !== initialFreeTextHighlightColor);

  const commitContents = async () => {
    if (annot.subtype === "FreeText" && onUpdateAnnotationProperties) {
      if ((localText === sourceText && !freeTextStyleChanged) || isSubmitting) return;
      setIsSubmitting(true);
      try {
        await onUpdateAnnotationProperties(annot.id, {
          ...(localText !== sourceText ? { contents: localText } : {}),
          ...(freeTextFamily !== (annot.fontFamily ?? "SansSerif")
            ? { font_family: freeTextFamily }
            : {}),
          ...(Math.abs(freeTextSize - (annot.fontSize ?? 12)) > 0.001
            ? { font_size: freeTextSize }
            : {}),
          ...(freeTextBold !== Boolean(annot.isBold) ? { bold: freeTextBold } : {}),
          ...(freeTextItalic !== Boolean(annot.isItalic) ? { italic: freeTextItalic } : {}),
          ...(freeTextColor !== initialTextColor
            ? { text_color: hexToRgb(freeTextColor) }
            : {}),
          ...(freeTextUnderlined !== Boolean(annot.isUnderlined)
            ? { underline: freeTextUnderlined }
            : {}),
          ...(freeTextStruckThrough !== Boolean(annot.isStruckThrough)
            ? { strikethrough: freeTextStruckThrough }
            : {}),
          ...(freeTextHighlighted !== Boolean(annot.highlightColor)
            ? { highlight_enabled: freeTextHighlighted }
            : {}),
          ...(freeTextHighlighted && freeTextHighlightColor !== initialFreeTextHighlightColor
            ? { highlight_color: hexToRgbTuple(freeTextHighlightColor) }
            : {}),
        });
      } finally {
        setIsSubmitting(false);
      }
      return;
    }
    if (!onAnnotationChange || localText === sourceText || isSubmitting) return;
    setIsSubmitting(true);
    try {
      await onAnnotationChange(annot.id, localText);
    } finally {
      setIsSubmitting(false);
    }
  };

  const hexToRgb = (hex: string): [number, number, number] => {
    const clean = hex.replace("#", "");
    return [
      parseInt(clean.substring(0, 2), 16) / 255,
      parseInt(clean.substring(2, 4), 16) / 255,
      parseInt(clean.substring(4, 6), 16) / 255,
    ];
  };

  const handleUpdateProps = async (update: StarPdfUpdateAnnotationInput) => {
    if (!onUpdateAnnotationProperties) return;
    setIsSubmitting(true);
    try {
      await onUpdateAnnotationProperties(annot.id, update);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (annot.subtype === "Link") {
    return (
      <div className="flex items-center gap-2 text-xs text-slate-600" data-testid="context-annotation-controls">
        <Badge variant="secondary" className="bg-purple-100 text-purple-700 font-normal shrink-0" data-testid="context-annotation-type">
          Link
        </Badge>
        <span className="truncate max-w-[260px] text-slate-500 italic" title={annot.contents || "Interactive Link"}>
          {annot.contents ? `Destination: ${annot.contents}` : "Interactive Link Destination (Read-Only)"}
        </span>
        {onDeleteAnnotation && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => void onDeleteAnnotation(annot.id)}
            disabled={isSubmitting}
            className="h-8 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50 shrink-0"
            title="Delete link annotation"
            data-testid="context-annotation-delete-btn"
          >
            <Trash2 className="size-3.5" />
            <span>Delete</span>
          </Button>
        )}
      </div>
    );
  }

  if (annot.subtype === "Square" || annot.subtype === "Circle") {
    return (
      <div className="flex items-center gap-2 min-w-0" data-testid="context-annotation-controls">
        <Badge variant="secondary" className="bg-purple-100 text-purple-700 font-normal shrink-0" data-testid="context-annotation-type">
          {annot.subtype}
        </Badge>
        <div className="flex items-center gap-1">
          <span className="text-[11px] text-slate-500">Stroke:</span>
          <input
            type="color"
            value={strokeColor}
            onChange={(e) => {
              setStrokeColor(e.target.value);
              void handleUpdateProps({ color: hexToRgb(e.target.value), border_width: borderWidth });
            }}
            className="size-6 rounded border border-slate-300 cursor-pointer p-0"
            title="Stroke Color"
            data-testid="context-annotation-stroke-color"
          />
        </div>
        <div className="flex items-center gap-1">
          <span className="text-[11px] text-slate-500">Fill:</span>
          <input
            type="color"
            value={fillColor}
            onChange={(e) => {
              setFillColor(e.target.value);
              void handleUpdateProps({ fill_color: hexToRgb(e.target.value) });
            }}
            className="size-6 rounded border border-slate-300 cursor-pointer p-0"
            title="Fill Color"
            data-testid="context-annotation-fill-color"
          />
        </div>
        <div className="flex items-center gap-1">
          <span className="text-[11px] text-slate-500">Width:</span>
          <select
            value={borderWidth}
            onChange={(e) => {
              const val = parseFloat(e.target.value);
              setBorderWidth(val);
              void handleUpdateProps({ border_width: val });
            }}
            className="h-7 text-xs rounded border border-slate-300 bg-white px-1.5 text-slate-700"
            data-testid="context-annotation-border-width"
          >
            <option value="0.5">0.5 pt</option>
            <option value="1.0">1.0 pt</option>
            <option value="1.5">1.5 pt</option>
            <option value="2.0">2.0 pt</option>
            <option value="3.0">3.0 pt</option>
            <option value="5.0">5.0 pt</option>
          </select>
        </div>
        {onDeleteAnnotation && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => void onDeleteAnnotation(annot.id)}
            disabled={isSubmitting}
            className="h-8 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50 shrink-0"
            title="Delete annotation"
            data-testid="context-annotation-delete-btn"
          >
            <Trash2 className="size-3.5" />
            <span>Delete</span>
          </Button>
        )}
      </div>
    );
  }

  if (annot.subtype === "Ink") {
    return (
      <div className="flex items-center gap-2 min-w-0" data-testid="context-annotation-controls">
        <Badge variant="secondary" className="bg-purple-100 text-purple-700 font-normal shrink-0" data-testid="context-annotation-type">
          Drawing
        </Badge>
        <div className="flex items-center gap-1">
          <span className="text-[11px] text-slate-500">Color:</span>
          <input
            type="color"
            value={strokeColor}
            onChange={(e) => {
              setStrokeColor(e.target.value);
              void handleUpdateProps({ color: hexToRgb(e.target.value) });
            }}
            className="size-6 rounded border border-slate-300 cursor-pointer p-0"
            title="Stroke Color"
            data-testid="context-annotation-ink-color"
          />
        </div>
        <div className="flex items-center gap-1">
          <span className="text-[11px] text-slate-500">Width:</span>
          <select
            value={borderWidth}
            onChange={(e) => {
              const val = parseFloat(e.target.value);
              setBorderWidth(val);
              void handleUpdateProps({ border_width: val });
            }}
            className="h-7 text-xs rounded border border-slate-300 bg-white px-1.5 text-slate-700"
            data-testid="context-annotation-ink-width"
          >
            <option value="1.0">1.0 pt</option>
            <option value="2.0">2.0 pt</option>
            <option value="3.0">3.0 pt</option>
            <option value="5.0">5.0 pt</option>
          </select>
        </div>
        {onDeleteAnnotation && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => void onDeleteAnnotation(annot.id)}
            disabled={isSubmitting}
            className="h-8 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50 shrink-0"
            title="Delete drawing"
            data-testid="context-annotation-delete-btn"
          >
            <Trash2 className="size-3.5" />
            <span>Delete</span>
          </Button>
        )}
      </div>
    );
  }

  if (annot.subtype === "Highlight") {
    return (
      <div className="flex items-center gap-2 min-w-0" data-testid="context-annotation-controls">
        <Badge variant="secondary" className="bg-purple-100 text-purple-700 font-normal shrink-0" data-testid="context-annotation-type">
          Highlight
        </Badge>
        <div className="flex items-center gap-1">
          <span className="text-[11px] text-slate-500">Color:</span>
          <input
            type="color"
            value={highlightColor}
            onChange={(e) => {
              setHighlightColor(e.target.value);
              void handleUpdateProps({ color: hexToRgb(e.target.value) });
            }}
            className="size-6 rounded border border-slate-300 cursor-pointer p-0"
            title="Highlight Color"
            data-testid="context-annotation-highlight-color"
          />
        </div>
        {onDeleteAnnotation && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => void onDeleteAnnotation(annot.id)}
            disabled={isSubmitting}
            className="h-8 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50 shrink-0"
            title="Delete highlight"
            data-testid="context-annotation-delete-btn"
          >
            <Trash2 className="size-3.5" />
            <span>Delete</span>
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-wrap items-center gap-2 min-w-0" data-testid="context-annotation-controls">
      <Badge variant="secondary" className="bg-purple-100 text-purple-700 font-normal shrink-0" data-testid="context-annotation-type">
        {annot.subtype}
      </Badge>
      <Input
        type="text"
        value={localText}
        onChange={(e) => setLocalText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            void commitContents();
          }
          if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            setLocalText(sourceText);
            setFreeTextFamily(annot.fontFamily ?? "SansSerif");
            setFreeTextSize(annot.fontSize ?? 12);
            setFreeTextBold(Boolean(annot.isBold));
            setFreeTextItalic(Boolean(annot.isItalic));
            setFreeTextColor(initialTextColor);
            setFreeTextUnderlined(Boolean(annot.isUnderlined));
            setFreeTextStruckThrough(Boolean(annot.isStruckThrough));
            setFreeTextHighlighted(Boolean(annot.highlightColor));
            setFreeTextHighlightColor(initialFreeTextHighlightColor);
            onDeselect?.();
          }
        }}
        className="h-8 text-xs w-48 sm:w-64 bg-white"
        placeholder="Annotation contents..."
        data-testid="context-annotation-input"
        autoFocus
      />
      {annot.subtype === "FreeText" && (
        <>
          <select
            value={freeTextFamily}
            onChange={(event) => setFreeTextFamily(event.target.value as typeof freeTextFamily)}
            className="h-8 rounded border border-slate-300 bg-white px-1.5 text-xs"
            aria-label="Font family"
            data-testid="context-freetext-font-family"
          >
            <option value="SansSerif">Helvetica</option>
            <option value="Serif">Times</option>
            <option value="Monospace">Courier</option>
          </select>
          <Input
            type="number"
            min={STARPDF_TEXT_FONT_SIZE_MIN}
            max={STARPDF_TEXT_FONT_SIZE_MAX}
            step={0.5}
            value={freeTextSize}
            onChange={(event) => setFreeTextSize(Number(event.target.value))}
            className="h-8 w-16 px-1.5 text-xs"
            aria-label="Font size"
            data-testid="context-freetext-font-size"
          />
          <Button
            type="button"
            variant={freeTextBold ? "default" : "outline"}
            size="icon"
            onClick={() => setFreeTextBold((value) => !value)}
            className="size-8"
            aria-label="Bold"
            aria-pressed={freeTextBold}
            data-testid="context-freetext-bold"
          >
            <BoldIcon className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant={freeTextItalic ? "default" : "outline"}
            size="icon"
            onClick={() => setFreeTextItalic((value) => !value)}
            className="size-8"
            aria-label="Italic"
            aria-pressed={freeTextItalic}
            data-testid="context-freetext-italic"
          >
            <ItalicIcon className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant={freeTextUnderlined ? "default" : "outline"}
            size="icon"
            onClick={() => setFreeTextUnderlined((value) => !value)}
            className="size-8"
            aria-label="Underline"
            aria-pressed={freeTextUnderlined}
            data-testid="context-freetext-underline"
          >
            <UnderlineIcon className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant={freeTextStruckThrough ? "default" : "outline"}
            size="icon"
            onClick={() => setFreeTextStruckThrough((value) => !value)}
            className="size-8"
            aria-label="Strikethrough"
            aria-pressed={freeTextStruckThrough}
            data-testid="context-freetext-strikethrough"
          >
            <StrikethroughIcon className="size-3.5" />
          </Button>
          <input
            type="color"
            value={freeTextColor}
            onChange={(event) => setFreeTextColor(event.target.value)}
            className="size-8 rounded border border-slate-300 bg-white p-0.5"
            aria-label="Text color"
            data-testid="context-freetext-color"
          />
          <Button
            type="button"
            variant={freeTextHighlighted ? "default" : "outline"}
            size="icon"
            onClick={() => setFreeTextHighlighted((value) => !value)}
            className="size-8"
            aria-label="Highlight"
            aria-pressed={freeTextHighlighted}
            data-testid="context-freetext-highlight"
          >
            <Highlighter className="size-3.5" />
          </Button>
          {freeTextHighlighted && (
            <input
              type="color"
              value={freeTextHighlightColor}
              onChange={(event) => setFreeTextHighlightColor(event.target.value)}
              className="size-8 rounded border border-slate-300 bg-white p-0.5"
              aria-label="Highlight color"
              data-testid="context-freetext-highlight-color"
            />
          )}
        </>
      )}
      <Button
        type="button"
        variant="outline"
        size="sm"
        onClick={() => void commitContents()}
        disabled={isSubmitting || (localText === sourceText && !freeTextStyleChanged)}
        className="h-8 text-xs px-2 shrink-0"
        data-testid="context-annotation-apply-btn"
      >
        Apply
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={() => {
          setLocalText("");
        }}
        disabled={isSubmitting || localText.length === 0}
        className="h-8 text-xs px-2 text-slate-500 hover:text-slate-700 shrink-0"
        title="Clear annotation contents"
        data-testid="context-annotation-clear-btn"
      >
        Clear
      </Button>

      {onDeleteAnnotation && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => void onDeleteAnnotation(annot.id)}
          disabled={isSubmitting}
          className="h-8 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50 shrink-0"
          title="Delete annotation"
          data-testid="context-annotation-delete-btn"
        >
          <Trash2 className="size-3.5" />
          <span>Delete</span>
        </Button>
      )}
    </div>
  );
}

export function PdfContextualToolbar({
  selection,
  containerRef,
  onDeselect,
  onReplaceText,
  onApplyTextStyle,
  onDeleteText,
  onReplaceImage,
  onRemoveImage,
  onUpdateGraphic,
  onDeleteGraphic,
  onFormFieldChange,
  formFieldValue,
  onAnnotationChange,
  annotationValue,
  onUpdateAnnotationProperties,
  onDeleteAnnotation,
  onAddTextInstead,
}: PdfContextualToolbarProps) {
  const toolbarRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<{
    top: number;
    left: number;
    visible: boolean;
  }>({
    top: 12,
    left: 12,
    visible: true,
  });

  const updatePosition = useCallback(() => {
    if (!selection || !selection.bounds) return;

    const container = containerRef?.current;
    if (!container) return;

    const containerRect = container.getBoundingClientRect();
    const canvasOverlay = container.querySelector(
      "[data-testid='pdf-interactive-overlay']",
    ) as HTMLElement | null;
    const canvasElem = (canvasOverlay || container.querySelector("canvas")) as HTMLElement | null;

    if (!canvasElem) return;

    const canvasRect = canvasElem.getBoundingClientRect();

    // Selected item position in screen coordinates
    const itemScreenLeft = canvasRect.left + selection.bounds.left;
    const itemScreenTop = canvasRect.top + selection.bounds.top;
    const itemWidth = selection.bounds.width;
    const itemHeight = selection.bounds.height;

    // Selected item position relative to viewport container
    const relLeft = itemScreenLeft - containerRect.left;
    const relTop = itemScreenTop - containerRect.top;
    const relRight = relLeft + itemWidth;
    const relBottom = relTop + itemHeight;

    // Hide if scrolled completely offscreen
    if (
      relBottom < -10 ||
      relTop > containerRect.height + 10 ||
      relRight < -10 ||
      relLeft > containerRect.width + 10
    ) {
      setPosition((prev) => (prev.visible ? { ...prev, visible: false } : prev));
      return;
    }

    const toolbarWidth = toolbarRef.current?.offsetWidth || 380;
    const toolbarHeight = toolbarRef.current?.offsetHeight || 48;
    const gap = 10;

    // Prefer above selection: relTop - toolbarHeight - gap
    let targetTop = relTop - toolbarHeight - gap;

    // Flip below if not enough room above
    if (targetTop < 8) {
      targetTop = relBottom + gap;
    }

    // Clamp top inside viewport
    targetTop = Math.max(8, Math.min(targetTop, containerRect.height - toolbarHeight - 8));

    // Center horizontally on selected object
    let targetLeft = relLeft + itemWidth / 2 - toolbarWidth / 2;

    // Clamp left inside viewport
    targetLeft = Math.max(8, Math.min(targetLeft, containerRect.width - toolbarWidth - 8));

    setPosition({
      top: Math.round(targetTop),
      left: Math.round(targetLeft),
      visible: true,
    });
  }, [selection, containerRef]);

  useEffect(() => {
    updatePosition();

    const container = containerRef?.current;
    if (!container) return;

    const handleScroll = () => {
      updatePosition();
    };
    const handleResize = () => {
      updatePosition();
    };

    container.addEventListener("scroll", handleScroll, { passive: true });
    window.addEventListener("resize", handleResize);

    return () => {
      container.removeEventListener("scroll", handleScroll);
      window.removeEventListener("resize", handleResize);
    };
  }, [updatePosition, containerRef]);

  if (!selection) return null;

  return (
    <div
      ref={toolbarRef}
      onClick={(e) => e.stopPropagation()}
      style={{
        top: `${position.top}px`,
        left: `${position.left}px`,
        opacity: position.visible ? 1 : 0,
        pointerEvents: position.visible ? "auto" : "none",
        transition: "opacity 120ms ease-out, transform 75ms ease-out",
      }}
      className="absolute z-30 max-w-2xl w-[92%] sm:w-auto bg-white/95 backdrop-blur-md rounded-2xl border border-slate-200/90 shadow-xl p-2.5 flex items-center gap-2.5 select-none"
      data-testid="pdf-contextual-toolbar"
      role="region"
      aria-label="Contextual edit toolbar"
    >

      {/* Selection Type Badge */}
      <div className="flex items-center gap-1.5 pl-1.5 pr-2 border-r border-slate-200 shrink-0">
        {selection.type === "text" && <Type className="size-4 text-sky-600" />}
        {selection.type === "image" && <ImageIcon className="size-4 text-emerald-600" />}
        {selection.type === "vector" && <Shapes className="size-4 text-indigo-600" />}
        {selection.type === "form" && <FileEdit className="size-4 text-amber-600" />}
        {selection.type === "annotation" && <FileEdit className="size-4 text-purple-600" />}
        <span className="text-xs font-semibold text-slate-700 capitalize">
          {selection.type === "text"
            ? "Text"
            : selection.type === "vector"
            ? "Shape"
            : selection.type === "annotation"
            ? "Annotation"
            : selection.type}
        </span>
      </div>

      {/* TEXT SELECTION CONTROLS */}
      {selection.type === "text" && (
        <TextControls
          key={`${selection.id}:${(selection.data as StarPdfTextSpan).text}:${(selection.data as StarPdfTextSpan).font_family ?? ""}:${(selection.data as StarPdfTextSpan).font_size}:${Boolean((selection.data as StarPdfTextSpan).is_bold)}:${Boolean((selection.data as StarPdfTextSpan).is_italic)}:${(selection.data as StarPdfTextSpan).fill_color?.join(",") ?? ""}:${Boolean((selection.data as StarPdfTextSpan).underline)}:${Boolean((selection.data as StarPdfTextSpan).strikethrough)}:${(selection.data as StarPdfTextSpan).highlight_color?.join(",") ?? ""}`}
          span={selection.data as StarPdfTextSpan}
          group={selection.group}
          onReplaceText={onReplaceText}
          onApplyTextStyle={onApplyTextStyle}
          onDeleteText={onDeleteText}
          onDeselect={onDeselect}
          onAddTextInstead={onAddTextInstead}
        />
      )}


      {/* IMAGE SELECTION CONTROLS */}
      {selection.type === "image" && (
        <ImageControls
          key={selection.id}
          img={selection.data as StarPdfImageInfo}
          onReplaceImage={onReplaceImage}
          onRemoveImage={onRemoveImage}
        />
      )}

      {/* VECTOR GRAPHIC SELECTION CONTROLS */}
      {selection.type === "vector" && (
        <VectorControls
          key={selection.id}
          graphic={selection.data as StarPdfVectorGraphicInfo}
          onUpdateGraphic={onUpdateGraphic}
          onDeleteGraphic={onDeleteGraphic}
        />
      )}

      {/* FORM FIELD SELECTION CONTROLS */}
      {selection.type === "form" && (() => {
        const field = selection.data as AcroFormField;
        const currentVal = formFieldValue ?? field.value;

        if (field.isReadOnly) {
          return (
            <div className="flex items-center gap-2 text-xs text-slate-600">
              <Badge variant="secondary" className="bg-slate-100 text-slate-700 gap-1 font-normal" data-testid="context-form-readonly">
                <Lock className="size-3 text-slate-400" /> Read-Only Field
              </Badge>
              <span className="truncate max-w-[200px]" title={field.name}>
                {field.name}
              </span>
            </div>
          );
        }

        if (field.type === "unsupported") {
          return (
            <div className="flex items-center gap-2 text-xs text-slate-600">
              <Badge variant="secondary" className="bg-slate-100 text-slate-700 gap-1 font-normal" data-testid="context-form-unsupported">
                <Lock className="size-3 text-slate-400" /> Unsupported Field
              </Badge>
              <span className="truncate max-w-[200px]" title={field.name}>
                {field.name}
              </span>
            </div>
          );
        }

        if (field.type === "checkbox") {
          const checked = Boolean(currentVal);
          return (
            <label className="flex items-center gap-2 cursor-pointer text-xs text-slate-700">
              <input
                type="checkbox"
                checked={checked}
                onChange={(e) => onFormFieldChange(field.name, e.target.checked)}
                className="size-4 accent-sky-600 rounded"
                data-testid="context-form-checkbox"
              />
              <span>{checked ? "Checked" : "Unchecked"}</span>
            </label>
          );
        }

        if (field.type === "radio") {
          return (
            <div className="flex items-center gap-2 text-xs text-slate-700" data-testid="context-form-radio-group">
              {field.options && field.options.length > 0 ? (
                field.options.map((opt) => (
                  <label key={opt} className="flex items-center gap-1 cursor-pointer">
                    <input
                      type="radio"
                      name={field.name}
                      value={opt}
                      checked={currentVal === opt}
                      onChange={() => onFormFieldChange(field.name, opt)}
                      className="size-3.5 accent-amber-600"
                      data-testid={`context-form-radio-${opt}`}
                    />
                    <span>{opt}</span>
                  </label>
                ))
              ) : (
                <label className="flex items-center gap-1.5 cursor-pointer">
                  <input
                    type="radio"
                    checked={Boolean(currentVal)}
                    onChange={() => onFormFieldChange(field.name, !currentVal)}
                    className="size-3.5 accent-amber-600"
                    data-testid="context-form-radio"
                  />
                  <span>{currentVal ? "Selected" : "Unselected"}</span>
                </label>
              )}
              {Boolean(currentVal) && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => onFormFieldChange(field.name, "")}
                  className="h-8 text-xs px-2 text-slate-500 hover:text-slate-700 shrink-0"
                  title="Clear selection"
                  data-testid="context-form-clear-btn"
                >
                  Clear
                </Button>
              )}
            </div>
          );
        }

        if (field.type === "dropdown" || field.type === "optionList") {
          return (
            <div className="flex items-center gap-1.5">
              <select
                value={String(currentVal)}
                onChange={(e) => onFormFieldChange(field.name, e.target.value)}
                className="h-8 text-xs rounded border border-slate-300 bg-white px-2 text-slate-700"
                data-testid="context-form-select"
              >
                {field.options?.map((opt) => (
                  <option key={opt} value={opt}>
                    {opt}
                  </option>
                ))}
              </select>
              {Boolean(currentVal) && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => onFormFieldChange(field.name, "")}
                  className="h-8 text-xs px-2 text-slate-500 hover:text-slate-700 shrink-0"
                  title="Clear field value"
                  data-testid="context-form-clear-btn"
                >
                  Clear
                </Button>
              )}
            </div>
          );
        }

        // Default text-like form field
        return (
          <div className="flex items-center gap-1.5">
            <Input
              type="text"
              value={typeof currentVal === "string" ? currentVal : ""}
              onChange={(e) => onFormFieldChange(field.name, e.target.value)}
              disabled={field.isReadOnly}
              className="h-8 text-xs w-48 sm:w-64 bg-white"
              placeholder="Field value..."
              data-testid="context-form-input"
            />
            {!field.isReadOnly && Boolean(currentVal) && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => onFormFieldChange(field.name, "")}
                className="h-8 text-xs px-2 text-slate-500 hover:text-slate-700 shrink-0"
                title="Clear field value"
                data-testid="context-form-clear-btn"
              >
                Clear
              </Button>
            )}
          </div>
        );
      })()}


      {/* MARKUP ANNOTATION SELECTION CONTROLS */}
      {selection.type === "annotation" && (
        <AnnotationControls
          key={`${selection.id}:${annotationValue ?? (selection.data as PdfMarkupAnnotation).contents ?? ""}:${(selection.data as PdfMarkupAnnotation).fontFamily ?? ""}:${(selection.data as PdfMarkupAnnotation).fontSize ?? ""}:${Boolean((selection.data as PdfMarkupAnnotation).isBold)}:${Boolean((selection.data as PdfMarkupAnnotation).isItalic)}:${(selection.data as PdfMarkupAnnotation).textColor?.join(",") ?? ""}:${Boolean((selection.data as PdfMarkupAnnotation).isUnderlined)}:${Boolean((selection.data as PdfMarkupAnnotation).isStruckThrough)}:${(selection.data as PdfMarkupAnnotation).highlightColor?.join(",") ?? ""}`}
          annot={selection.data as PdfMarkupAnnotation}
          annotationValue={annotationValue}
          onAnnotationChange={onAnnotationChange}
          onUpdateAnnotationProperties={onUpdateAnnotationProperties}
          onDeleteAnnotation={onDeleteAnnotation}
          onDeselect={onDeselect}
        />
      )}

      {/* Close / Deselect Button */}
      <Button
        type="button"
        variant="ghost"
        size="icon"
        onClick={onDeselect}
        className="size-7 text-slate-400 hover:text-slate-700 rounded-lg ml-auto shrink-0"
        title="Close Selection (Esc)"
        aria-label="Close selection"
      >
        <X className="size-3.5" />
      </Button>
    </div>
  );
}
