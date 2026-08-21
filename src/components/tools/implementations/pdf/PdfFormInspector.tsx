"use client";

import { useId, useState, useRef } from "react";
import { type AcroFormField } from "@/lib/pdf/pdf-types";
import type {
  StarPdfImageInfo,
  StarPdfTextSpan,
  StarPdfVectorGraphicInfo,
  StarPdfUpdateVectorGraphicInput,
} from "@/lib/pdf/starpdf-types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import {
  RotateCcw,
  FileEdit,
  Type,
  ImageIcon,
  Shapes,
  CheckCircle2,
  AlertTriangle,
  ArrowRight,
  Upload,
  Trash2,
  RefreshCw,
  Plus,
} from "lucide-react";
import { cn } from "@/lib/utils";

interface PdfFormInspectorProps {
  fields: AcroFormField[];
  fieldValues: Record<string, string | boolean | string[]>;
  onFieldValueChange: (name: string, value: string | boolean | string[]) => void;
  onResetForm: () => void;
  isModified: boolean;
  textSpans?: StarPdfTextSpan[];
  onReplaceText?: (spanId: string, newText: string) => Promise<void>;
  images?: StarPdfImageInfo[];
  onReplaceImage?: (imageId: string, file: File) => Promise<void>;
  onAddImage?: (file: File, x: number, y: number, width: number, height: number) => Promise<void>;
  onRemoveImage?: (imageId: string) => Promise<void>;
  graphics?: StarPdfVectorGraphicInfo[];
  onUpdateGraphic?: (input: StarPdfUpdateVectorGraphicInput) => Promise<void>;
  onAddRectangle?: (
    x: number,
    y: number,
    width: number,
    height: number,
    strokeColorHex?: string,
    fillColorHex?: string,
    lineWidth?: number,
  ) => Promise<void>;
  onAddLine?: (
    x1: number,
    y1: number,
    x2: number,
    y2: number,
    strokeColorHex?: string,
    lineWidth?: number,
  ) => Promise<void>;
  onDeleteGraphic?: (graphicId: string) => Promise<void>;
  className?: string;
}

export function PdfFormInspector({
  fields,
  fieldValues,
  onFieldValueChange,
  onResetForm,
  isModified,
  textSpans = [],
  onReplaceText,
  images = [],
  onReplaceImage,
  onAddImage,
  onRemoveImage,
  graphics = [],
  onUpdateGraphic,
  onAddRectangle,
  onAddLine,
  onDeleteGraphic,
  className = "",
}: PdfFormInspectorProps) {
  const baseId = useId();
  const [activeTab, setActiveTab] = useState<"fields" | "text" | "images" | "shapes">(() =>
    fields.length > 0 ? "fields" : "text",
  );

  const [selectedSpanId, setSelectedSpanId] = useState<string | null>(null);
  const [replacementText, setReplacementText] = useState<string>("");
  const [isReplacing, setIsReplacing] = useState<boolean>(false);
  const [isImageProcessing, setIsImageProcessing] = useState<boolean>(false);
  const [isVectorProcessing, setIsVectorProcessing] = useState<boolean>(false);

  // Vector graphics selection & edit state
  const [selectedGraphicId, setSelectedGraphicId] = useState<string | null>(null);
  const [editLineWidth, setEditLineWidth] = useState<number>(1.0);
  const [editStrokeColor, setEditStrokeColor] = useState<string>("#000000");
  const [editFillColor, setEditFillColor] = useState<string>("#3b82f6");
  const [editIsStroked, setEditIsStroked] = useState<boolean>(true);
  const [editIsFilled, setEditIsFilled] = useState<boolean>(false);

  const addImageInputRef = useRef<HTMLInputElement | null>(null);
  const replaceImageInputRef = useRef<HTMLInputElement | null>(null);
  const [targetReplaceImageId, setTargetReplaceImageId] = useState<string | null>(null);

  const handleSelectSpan = (span: StarPdfTextSpan) => {
    setSelectedSpanId(span.span_id);
    setReplacementText(span.text);
  };

  const handleApplyTextEdit = async () => {
    if (!selectedSpanId || !onReplaceText) return;
    setIsReplacing(true);
    try {
      await onReplaceText(selectedSpanId, replacementText);
    } finally {
      setIsReplacing(false);
    }
  };

  const handleSelectGraphic = (graphic: StarPdfVectorGraphicInfo) => {
    setSelectedGraphicId(graphic.graphic_id);
    setEditLineWidth(graphic.line_width || 1.0);
    setEditStrokeColor(graphic.stroke_color_hex || "#000000");
    setEditFillColor(graphic.fill_color_hex || "#3b82f6");
    setEditIsStroked(graphic.is_stroked);
    setEditIsFilled(graphic.is_filled);
  };

  const hexToRgb = (hex: string): [number, number, number] => {
    const clean = hex.replace("#", "");
    if (clean.length === 6) {
      const r = parseInt(clean.substring(0, 2), 16) / 255;
      const g = parseInt(clean.substring(2, 4), 16) / 255;
      const b = parseInt(clean.substring(4, 6), 16) / 255;
      return [r, g, b];
    }
    return [0, 0, 0];
  };

  const handleApplyGraphicEdit = async () => {
    if (!selectedGraphicId || !onUpdateGraphic) return;
    const graphic = graphics.find((g) => g.graphic_id === selectedGraphicId);
    if (!graphic) return;

    setIsVectorProcessing(true);
    try {
      const strokeRgb = editIsStroked ? hexToRgb(editStrokeColor) : undefined;
      const fillRgb = editIsFilled ? hexToRgb(editFillColor) : undefined;

      await onUpdateGraphic({
        page_index: graphic.page_index,
        graphic_id: selectedGraphicId,
        stroke_color_rgb: strokeRgb,
        fill_color_rgb: fillRgb,
        line_width: editLineWidth,
        is_stroked: editIsStroked,
        is_filled: editIsFilled,
        clone_if_shared: true,
      });
    } finally {
      setIsVectorProcessing(false);
    }
  };

  const handleDeleteGraphicClick = async (graphicId: string) => {
    if (!onDeleteGraphic) return;
    setIsVectorProcessing(true);
    try {
      await onDeleteGraphic(graphicId);
      if (selectedGraphicId === graphicId) {
        setSelectedGraphicId(null);
      }
    } finally {
      setIsVectorProcessing(false);
    }
  };

  const handleTriggerReplaceImage = (imageId: string) => {
    setTargetReplaceImageId(imageId);
    replaceImageInputRef.current?.click();
  };

  const handleReplaceImageFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !targetReplaceImageId || !onReplaceImage) return;

    setIsImageProcessing(true);
    try {
      await onReplaceImage(targetReplaceImageId, file);
    } finally {
      setIsImageProcessing(false);
      if (replaceImageInputRef.current) replaceImageInputRef.current.value = "";
      setTargetReplaceImageId(null);
    }
  };

  const handleAddImageFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !onAddImage) return;

    setIsImageProcessing(true);
    try {
      await onAddImage(file, 100, 100, 200, 150);
    } finally {
      setIsImageProcessing(false);
      if (addImageInputRef.current) addImageInputRef.current.value = "";
    }
  };

  const handleRemoveImageClick = async (imageId: string) => {
    if (!onRemoveImage) return;
    setIsImageProcessing(true);
    try {
      await onRemoveImage(imageId);
    } finally {
      setIsImageProcessing(false);
    }
  };

  return (
    <aside
      className={cn(
        "w-80 shrink-0 border-l border-slate-200 bg-white p-4 flex flex-col justify-between overflow-y-auto max-h-[750px] space-y-4",
        className,
      )}
      aria-label="Inspector Panel"
    >
      <div className="space-y-4">
        {/* Top Header */}
        <div className="flex items-center justify-between pb-2 border-b border-slate-100">
          <div>
            <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-1.5">
              <FileEdit className="size-4 text-sky-600" />
              Form Fields
            </h3>
            <p className="text-xs text-slate-500">{fields.length} interactive fields detected</p>
          </div>
          {isModified && (
            <Badge variant="outline" className="text-[10px] text-amber-700 bg-amber-50 border-amber-200">
              Edited
            </Badge>
          )}
        </div>

        {/* Tab Switcher */}
        <div className="flex items-center gap-1 p-1 bg-slate-100 rounded-lg">
          <button
            type="button"
            onClick={() => setActiveTab("fields")}
            className={cn(
              "flex-1 text-[11px] font-medium py-1.5 px-1 rounded-md transition-all flex items-center justify-center gap-1",
              activeTab === "fields"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900",
            )}
          >
            <FileEdit className="size-3" />
            Forms ({fields.length})
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("text")}
            className={cn(
              "flex-1 text-[11px] font-medium py-1.5 px-1 rounded-md transition-all flex items-center justify-center gap-1",
              activeTab === "text"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900",
            )}
          >
            <Type className="size-3" />
            Text ({textSpans.length})
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("shapes")}
            className={cn(
              "flex-1 text-[11px] font-medium py-1.5 px-1 rounded-md transition-all flex items-center justify-center gap-1",
              activeTab === "shapes"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900",
            )}
          >
            <Shapes className="size-3" />
            Shapes ({graphics.length})
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("images")}
            className={cn(
              "flex-1 text-[11px] font-medium py-1.5 px-1 rounded-md transition-all flex items-center justify-center gap-1",
              activeTab === "images"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900",
            )}
          >
            <ImageIcon className="size-3" />
            Images ({images.length})
          </button>
        </div>

        {activeTab === "shapes" ? (
          /* Shapes & Vectors List & Editor */
          <div className="space-y-3">
            <div className="flex items-center justify-between pb-1 border-b border-slate-100">
              <span className="text-xs font-semibold text-slate-800">
                Vector Shapes ({graphics.length})
              </span>
              <div className="flex items-center gap-1">
                {onAddRectangle && (
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => onAddRectangle(100, 100, 150, 80, "#2563eb", "#dbeafe", 1.5)}
                    disabled={isVectorProcessing}
                    className="h-6 text-[10px] px-1.5 border-slate-200 hover:bg-slate-100"
                    title="Add Rectangle"
                  >
                    <Plus className="size-3 mr-0.5" /> Rect
                  </Button>
                )}
                {onAddLine && (
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => onAddLine(50, 500, 250, 500, "#000000", 2.0)}
                    disabled={isVectorProcessing}
                    className="h-6 text-[10px] px-1.5 border-slate-200 hover:bg-slate-100"
                    title="Add Line"
                  >
                    <Plus className="size-3 mr-0.5" /> Line
                  </Button>
                )}
              </div>
            </div>

            {graphics.length === 0 ? (
              <p className="text-xs text-slate-400 italic text-center py-6">
                No vector shapes found on this page.
              </p>
            ) : (
              <div className="space-y-2 pr-1 max-h-[480px] overflow-y-auto">
                {graphics.map((g) => {
                  const isSelected = selectedGraphicId === g.graphic_id;
                  const isEditable = g.is_editable;

                  return (
                    <div
                      key={g.graphic_id}
                      onClick={() => handleSelectGraphic(g)}
                      className={cn(
                        "p-2.5 rounded-lg border text-left cursor-pointer transition-all space-y-2",
                        isSelected
                          ? "border-sky-500 bg-sky-50/40 ring-1 ring-sky-500"
                          : "border-slate-100 bg-slate-50/50 hover:bg-slate-100/50",
                      )}
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-semibold text-slate-800 flex items-center gap-1">
                          <Shapes className="size-3.5 text-sky-600" />
                          {g.graphic_type}
                        </span>
                        <div className="flex items-center gap-1">
                          {isEditable ? (
                            <Badge variant="outline" className="text-[10px] text-emerald-700 bg-emerald-50 border-emerald-200">
                              <CheckCircle2 className="size-2.5 mr-0.5" /> Editable
                            </Badge>
                          ) : (
                            <Badge variant="outline" className="text-[10px] text-amber-700 bg-amber-50 border-amber-200">
                              <AlertTriangle className="size-2.5 mr-0.5" /> {g.editability_code}
                            </Badge>
                          )}
                        </div>
                      </div>

                      <div className="text-[11px] text-slate-600 grid grid-cols-2 gap-1 bg-white p-1.5 rounded border border-slate-100">
                        <div>
                          <span className="text-slate-400">Stroke:</span>{" "}
                          {g.is_stroked ? g.stroke_color_hex || "Default" : "None"}
                        </div>
                        <div>
                          <span className="text-slate-400">Fill:</span>{" "}
                          {g.is_filled ? g.fill_color_hex || "Default" : "None"}
                        </div>
                        <div>
                          <span className="text-slate-400">Width:</span> {g.line_width} pt
                        </div>
                        <div>
                          <span className="text-slate-400">Bounds:</span> {Math.round(g.bounds[2] - g.bounds[0])}×{Math.round(g.bounds[3] - g.bounds[1])} pt
                        </div>
                      </div>

                      {isSelected && isEditable && (
                        <div
                          className="pt-2 border-t border-sky-200 space-y-2"
                          onClick={(e) => e.stopPropagation()}
                        >
                          <div className="grid grid-cols-2 gap-2">
                            <div className="space-y-1">
                              <Label className="text-[10px] text-slate-600">Stroke Color</Label>
                              <div className="flex items-center gap-1">
                                <input
                                  type="color"
                                  value={editStrokeColor}
                                  onChange={(e) => setEditStrokeColor(e.target.value)}
                                  className="size-6 p-0 border border-slate-200 rounded cursor-pointer"
                                />
                                <span className="text-[10px] font-mono text-slate-600">{editStrokeColor}</span>
                              </div>
                            </div>
                            <div className="space-y-1">
                              <Label className="text-[10px] text-slate-600">Fill Color</Label>
                              <div className="flex items-center gap-1">
                                <input
                                  type="color"
                                  value={editFillColor}
                                  onChange={(e) => setEditFillColor(e.target.value)}
                                  className="size-6 p-0 border border-slate-200 rounded cursor-pointer"
                                />
                                <span className="text-[10px] font-mono text-slate-600">{editFillColor}</span>
                              </div>
                            </div>
                          </div>

                          <div className="flex items-center justify-between pt-1">
                            <div className="flex items-center gap-2">
                              <Label className="text-[10px] text-slate-600">Line Width</Label>
                              <Input
                                type="number"
                                step="0.5"
                                min="0"
                                max="50"
                                value={editLineWidth}
                                onChange={(e) => setEditLineWidth(parseFloat(e.target.value) || 1.0)}
                                className="h-6 w-16 text-xs p-1 bg-white"
                              />
                            </div>
                            <div className="flex items-center gap-1">
                              <label className="text-[10px] text-slate-600 flex items-center gap-1 cursor-pointer">
                                <input
                                  type="checkbox"
                                  checked={editIsFilled}
                                  onChange={(e) => setEditIsFilled(e.target.checked)}
                                  className="size-3"
                                />
                                Fill
                              </label>
                            </div>
                          </div>

                          <div className="flex items-center gap-1.5 pt-1">
                            <Button
                              size="sm"
                              onClick={handleApplyGraphicEdit}
                              disabled={isVectorProcessing}
                              className="flex-1 h-7 text-xs bg-sky-600 hover:bg-sky-700 text-white flex items-center justify-center gap-1"
                            >
                              Apply Edit
                            </Button>
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => handleDeleteGraphicClick(g.graphic_id)}
                              disabled={isVectorProcessing}
                              className="h-7 text-xs border-red-200 text-red-600 hover:bg-red-50 hover:border-red-300 px-2"
                              title="Delete Shape"
                            >
                              <Trash2 className="size-3" />
                            </Button>
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        ) : activeTab === "text" ? (
          /* Text Spans List & Native Editor */
          <div className="space-y-3">
            <div className="flex items-center justify-between pb-1 border-b border-slate-100">
              <span className="text-xs font-semibold text-slate-800">
                Page Text ({textSpans.length} spans)
              </span>
              <span className="text-[10px] text-slate-400">Bounded v0.13</span>
            </div>

            {textSpans.length === 0 ? (
              <p className="text-xs text-slate-400 italic text-center py-6">
                No extractable text spans found on this page.
              </p>
            ) : (
              <div className="space-y-2 pr-1 max-h-[480px] overflow-y-auto">
                {textSpans.map((span) => {
                  const isSelected = selectedSpanId === span.span_id;
                  const isEditable = span.is_editable;

                  return (
                    <div
                      key={span.span_id}
                      onClick={() => handleSelectSpan(span)}
                      className={cn(
                        "p-2.5 rounded-lg border text-left cursor-pointer transition-all space-y-2",
                        isSelected
                          ? "border-sky-500 bg-sky-50/40 ring-1 ring-sky-500"
                          : "border-slate-100 bg-slate-50/50 hover:bg-slate-100/50",
                      )}
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-semibold text-slate-800 truncate max-w-[180px]">
                          {span.text}
                        </span>
                        {isEditable ? (
                          <Badge variant="outline" className="text-[10px] text-emerald-700 bg-emerald-50 border-emerald-200">
                            <CheckCircle2 className="size-2.5 mr-0.5" /> Editable
                          </Badge>
                        ) : (
                          <Badge variant="outline" className="text-[10px] text-amber-700 bg-amber-50 border-amber-200">
                            <AlertTriangle className="size-2.5 mr-0.5" /> {span.editability_code}
                          </Badge>
                        )}
                      </div>

                      <div className="text-[11px] text-slate-500 flex items-center gap-2">
                        <span>Font: {span.font_name}</span>
                        <span>·</span>
                        <span>Size: {span.font_size} pt</span>
                      </div>

                      {isSelected && isEditable && (
                        <div
                          className="pt-2 border-t border-sky-200 space-y-2"
                          onClick={(e) => e.stopPropagation()}
                        >
                          <Label className="text-[11px] text-slate-700 font-medium">Replace with:</Label>
                          <div className="flex items-center gap-1.5">
                            <Input
                              value={replacementText}
                              onChange={(e) => setReplacementText(e.target.value)}
                              className="h-8 text-xs bg-white"
                              placeholder="New text..."
                            />
                            <Button
                              size="sm"
                              onClick={handleApplyTextEdit}
                              disabled={isReplacing || replacementText === span.text}
                              className="h-8 text-xs bg-sky-600 hover:bg-sky-700 text-white flex items-center gap-1"
                            >
                              <ArrowRight className="size-3" />
                              Apply
                            </Button>
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        ) : activeTab === "images" ? (
          /* Images List & Actions */
          <div className="space-y-3">
            <div className="flex items-center justify-between pb-1 border-b border-slate-100">
              <span className="text-xs font-semibold text-slate-800">
                Page Images ({images.length})
              </span>
              <div>
                <input
                  type="file"
                  ref={addImageInputRef}
                  onChange={handleAddImageFileChange}
                  accept="image/jpeg,image/png"
                  className="hidden"
                />
                <input
                  type="file"
                  ref={replaceImageInputRef}
                  onChange={handleReplaceImageFileChange}
                  accept="image/jpeg,image/png"
                  className="hidden"
                />
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => addImageInputRef.current?.click()}
                  disabled={isImageProcessing}
                  className="h-6 text-[10px] px-2 border-slate-200 hover:bg-slate-100 flex items-center gap-1"
                >
                  <Upload className="size-3" />
                  Add Image
                </Button>
              </div>
            </div>

            {images.length === 0 ? (
              <p className="text-xs text-slate-400 italic text-center py-6">
                No image XObjects found on this page.
              </p>
            ) : (
              <div className="space-y-2 pr-1 max-h-[480px] overflow-y-auto">
                {images.map((img) => {
                  const widthPt = Math.round(img.rect[2] - img.rect[0]);
                  const heightPt = Math.round(img.rect[3] - img.rect[1]);

                  return (
                    <div
                      key={img.image_id}
                      className="p-2.5 rounded-lg border border-slate-100 bg-slate-50/50 space-y-2"
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-xs font-semibold text-slate-800 flex items-center gap-1">
                          <ImageIcon className="size-3.5 text-sky-600" />
                          {img.resource_name}
                        </span>
                        {img.is_nested_form ? (
                          <Badge variant="outline" className="text-[10px] text-amber-700 bg-amber-50 border-amber-200">
                            Nested Form
                          </Badge>
                        ) : (
                          <Badge variant="outline" className="text-[10px] text-emerald-700 bg-emerald-50 border-emerald-200">
                            Native Image
                          </Badge>
                        )}
                      </div>

                      <div className="text-[11px] text-slate-600 grid grid-cols-2 gap-1 bg-white p-1.5 rounded border border-slate-100">
                        <div>
                          <span className="text-slate-400">Pixels:</span> {img.width} × {img.height}
                        </div>
                        <div>
                          <span className="text-slate-400">Color:</span> {img.color_space}
                        </div>
                        <div>
                          <span className="text-slate-400">Size:</span> {widthPt} × {heightPt} pt
                        </div>
                        <div>
                          <span className="text-slate-400">Filter:</span> {img.filter || "Raw"}
                        </div>
                      </div>

                      <div className="flex items-center gap-1.5 pt-1">
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => handleTriggerReplaceImage(img.image_id)}
                          disabled={isImageProcessing || img.is_nested_form}
                          className="flex-1 h-7 text-xs border-slate-200 hover:bg-slate-100 flex items-center justify-center gap-1"
                        >
                          <RefreshCw className="size-3" />
                          Replace
                        </Button>
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => handleRemoveImageClick(img.image_id)}
                          disabled={isImageProcessing || img.is_nested_form}
                          className="h-7 text-xs border-red-200 text-red-600 hover:bg-red-50 hover:border-red-300 flex items-center justify-center px-2"
                          title="Remove image from page"
                        >
                          <Trash2 className="size-3" />
                        </Button>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        ) : (
          /* Form Fields List */
          <div className="space-y-4">
            {fields.length === 0 ? (
              <p className="text-xs text-slate-400 italic text-center py-6">
                No interactive form fields found in this PDF.
              </p>
            ) : (
              fields.map((field) => {
                const inputId = `${baseId}-${field.name}`;
                const val = fieldValues[field.name];

                return (
                  <div key={field.name} className="space-y-1.5 p-2.5 rounded-lg border border-slate-100 bg-slate-50/50">
                    <div className="flex items-center justify-between">
                      <Label htmlFor={inputId} className="text-xs font-medium text-slate-700 truncate max-w-[180px]">
                        {field.name}
                      </Label>
                      <Badge variant="secondary" className="text-[10px] px-1.5 py-0 capitalize">
                        {field.type}
                      </Badge>
                    </div>

                    {field.type === "text" && (
                      <Input
                        id={inputId}
                        value={typeof val === "string" ? val : ""}
                        onChange={(e) => onFieldValueChange(field.name, e.target.value)}
                        className="h-8 text-xs bg-white"
                        placeholder="Enter text..."
                      />
                    )}

                    {field.type === "checkbox" && (
                      <div className="flex items-center space-x-2 pt-1">
                        <Checkbox
                          id={inputId}
                          checked={Boolean(val)}
                          onCheckedChange={(checked) => onFieldValueChange(field.name, Boolean(checked))}
                        />
                        <label htmlFor={inputId} className="text-xs text-slate-600 cursor-pointer">
                          {Boolean(val) ? "Checked" : "Unchecked"}
                        </label>
                      </div>
                    )}

                    {field.type === "radio" && field.options && (
                      <div className="space-y-1 pt-1">
                        {field.options.map((opt) => (
                          <div key={opt} className="flex items-center space-x-2">
                            <input
                              type="radio"
                              id={`${inputId}-${opt}`}
                              name={field.name}
                              value={opt}
                              checked={val === opt}
                              onChange={() => onFieldValueChange(field.name, opt)}
                              className="size-3.5 text-sky-600 focus:ring-sky-500"
                            />
                            <label htmlFor={`${inputId}-${opt}`} className="text-xs text-slate-600 cursor-pointer">
                              {opt}
                            </label>
                          </div>
                        ))}
                      </div>
                    )}

                    {field.type === "dropdown" && field.options && (
                      <select
                        id={inputId}
                        value={typeof val === "string" ? val : ""}
                        onChange={(e) => onFieldValueChange(field.name, e.target.value)}
                        className="w-full h-8 text-xs rounded-md border border-slate-200 bg-white px-2 py-1 text-slate-800 focus:outline-none focus:ring-1 focus:ring-sky-500"
                      >
                        <option value="">Select option...</option>
                        {field.options.map((opt) => (
                          <option key={opt} value={opt}>
                            {opt}
                          </option>
                        ))}
                      </select>
                    )}
                  </div>
                );
              })
            )}
          </div>
        )}
      </div>

      {/* Footer Reset Action */}
      {fields.length > 0 && activeTab === "fields" && (
        <div className="pt-3 border-t border-slate-100">
          <Button
            variant="outline"
            size="sm"
            onClick={onResetForm}
            disabled={!isModified}
            className="w-full text-xs text-slate-600 hover:text-slate-900 flex items-center justify-center gap-1.5"
          >
            <RotateCcw className="size-3.5" />
            Reset to Original Values
          </Button>
        </div>
      )}
    </aside>
  );
}
