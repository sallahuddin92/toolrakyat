"use client";

import { useState, useRef } from "react";

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
} from "lucide-react";
import type {
  StarPdfTextSpan,
  StarPdfImageInfo,
  StarPdfVectorGraphicInfo,
  StarPdfUpdateVectorGraphicInput,
} from "@/lib/pdf/starpdf-types";
import type { AcroFormField, PdfMarkupAnnotation } from "@/lib/pdf/pdf-types";
import type { SmartPdfSelection, SelectionType } from "@/lib/pdf/selection";

export type { SelectionType };
export type SelectedItem = NonNullable<SmartPdfSelection>;

interface PdfContextualToolbarProps {
  selection: SmartPdfSelection;
  onDeselect: () => void;
  onReplaceText: (spanId: string, newText: string) => Promise<void>;
  onReplaceImage: (imageId: string, file: File) => Promise<void>;
  onRemoveImage: (imageId: string) => Promise<void>;
  onUpdateGraphic: (input: StarPdfUpdateVectorGraphicInput) => Promise<void>;
  onDeleteGraphic: (graphicId: string) => Promise<void>;
  onFormFieldChange: (fieldName: string, value: string | boolean | string[]) => void;
  formFieldValue?: string | boolean | string[];
  onAnnotationChange?: (annotId: string, value: string) => void;
  annotationValue?: string;
  onDeleteAnnotation?: (annotId: string) => Promise<void>;
  onAddTextInstead?: () => void;
}

function TextControls({
  span,
  group,
  onReplaceText,
  onDeselect,
  onAddTextInstead,
}: {
  span: StarPdfTextSpan;
  group?: import("@/lib/pdf/grouping").HumanTextGroup;
  onReplaceText: (spanId: string, newText: string) => Promise<void>;
  onDeselect: () => void;
  onAddTextInstead?: () => void;
}) {
  const [editText, setEditText] = useState(group?.text || span.text);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showDetails, setShowDetails] = useState(false);

  const isEditable = group ? group.editability === "EDITABLE_ATOMIC" : span.is_editable;

  if (!isEditable) {
    const isMultiSpan = group?.editability === "GROUP_SELECTION_ONLY";
    const detailedReason =
      group?.detailed_reason ||
      span.refusal_reason ||
      "This PDF uses a specialized embedded font mapping that StarPDF cannot safely rewrite for the requested characters. The original PDF was left unchanged.";

    return (
      <div className="flex items-center gap-2 text-xs text-slate-600 flex-wrap" data-testid="context-text-controls">
        <Badge variant="secondary" className="bg-slate-100 text-slate-700 gap-1 font-normal">
          <Lock className="size-3 text-slate-400" /> {isMultiSpan ? "Multi-Span" : "Read-Only"}
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
    if (!editText.trim()) return;
    setIsSubmitting(true);
    try {
      await onReplaceText(group?.primarySpanId || span.span_id, editText);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="flex items-center gap-2 min-w-0" data-testid="context-text-controls">
      <Input
        type="text"
        value={editText}
        onChange={(e) => setEditText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void handleTextSubmit();
          if (e.key === "Escape") onDeselect();
        }}
        className="h-8 text-xs w-48 sm:w-72 bg-white"
        placeholder="Edit text content..."
        data-testid="context-text-input"
        autoFocus
      />
      <Button
        type="button"
        size="sm"
        onClick={() => void handleTextSubmit()}
        disabled={isSubmitting || !editText.trim()}
        className="h-8 text-xs gap-1 bg-sky-600 hover:bg-sky-700 text-white"
        data-testid="context-text-save-btn"
      >
        <Check className="size-3.5" />
        <span>Apply</span>
      </Button>
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
  onDeleteAnnotation,
}: {
  annot: PdfMarkupAnnotation;
  annotationValue?: string;
  onAnnotationChange?: (annotId: string, value: string) => void;
  onDeleteAnnotation?: (annotId: string) => Promise<void>;
}) {
  const [localText, setLocalText] = useState<string | null>(null);
  const displayVal = localText ?? annotationValue ?? annot.contents ?? "";

  if (annot.subtype === "Link") {
    return (
      <div className="flex items-center gap-2 text-xs text-slate-600" data-testid="context-annotation-controls">
        <Badge variant="secondary" className="bg-purple-100 text-purple-700 font-normal shrink-0" data-testid="context-annotation-type">
          Link
        </Badge>
        <span className="truncate max-w-[260px] text-slate-500 italic">
          Interactive Link Destination (Read-Only)
        </span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 min-w-0" data-testid="context-annotation-controls">
      <Badge variant="secondary" className="bg-purple-100 text-purple-700 font-normal shrink-0" data-testid="context-annotation-type">
        {annot.subtype}
      </Badge>
      <Input
        type="text"
        value={displayVal}
        onChange={(e) => {
          setLocalText(e.target.value);
          onAnnotationChange?.(annot.id, e.target.value);
        }}
        className="h-8 text-xs w-48 sm:w-64 bg-white"
        placeholder="Annotation text / contents..."
        data-testid="context-annotation-input"
        autoFocus
      />

      {onDeleteAnnotation && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => void onDeleteAnnotation(annot.id)}
          className="h-8 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50"
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
  onDeselect,
  onReplaceText,
  onReplaceImage,
  onRemoveImage,
  onUpdateGraphic,
  onDeleteGraphic,
  onFormFieldChange,
  formFieldValue,
  onAnnotationChange,
  annotationValue,
  onDeleteAnnotation,
  onAddTextInstead,
}: PdfContextualToolbarProps) {

  if (!selection) return null;

  return (
    <div
      onClick={(e) => e.stopPropagation()}
      className="absolute top-3 left-1/2 -translate-x-1/2 z-30 max-w-2xl w-[92%] sm:w-auto bg-white/95 backdrop-blur-md rounded-2xl border border-slate-200/90 shadow-xl p-2.5 flex items-center gap-2.5 animate-in fade-in slide-in-from-top-2 duration-150"
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
          key={selection.id}
          span={selection.data as StarPdfTextSpan}
          group={selection.group}
          onReplaceText={onReplaceText}
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

        if (field.type === "dropdown" || field.type === "optionList") {
          return (
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
          );
        }

        // Default text-like form field
        return (
          <Input
            type="text"
            value={typeof currentVal === "string" ? currentVal : ""}
            onChange={(e) => onFormFieldChange(field.name, e.target.value)}
            disabled={field.isReadOnly}
            className="h-8 text-xs w-48 sm:w-64 bg-white"
            placeholder="Field value..."
            data-testid="context-form-input"
          />
        );
      })()}

      {/* MARKUP ANNOTATION SELECTION CONTROLS */}
      {selection.type === "annotation" && (
        <AnnotationControls
          key={selection.id}
          annot={selection.data as PdfMarkupAnnotation}
          annotationValue={annotationValue}
          onAnnotationChange={onAnnotationChange}
          onDeleteAnnotation={onDeleteAnnotation}
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
