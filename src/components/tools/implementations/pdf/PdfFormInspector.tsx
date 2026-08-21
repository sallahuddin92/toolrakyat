"use client";

import { useId, useState, useEffect, useRef } from "react";
import { type AcroFormField } from "@/lib/pdf/pdf-types";
import type { StarPdfImageInfo, StarPdfTextSpan } from "@/lib/pdf/starpdf-types";
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
  CheckCircle2,
  AlertTriangle,
  XCircle,
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
  className = "",
}: PdfFormInspectorProps) {
  const baseId = useId();
  const [activeTab, setActiveTab] = useState<"fields" | "text" | "images">(() =>
    fields.length > 0 ? "fields" : "text",
  );

  useEffect(() => {
    if (fields.length > 0) {
      setActiveTab("fields");
    }
  }, [fields.length]);

  const [selectedSpanId, setSelectedSpanId] = useState<string | null>(null);
  const [replacementText, setReplacementText] = useState<string>("");
  const [isReplacing, setIsReplacing] = useState<boolean>(false);
  const [isImageProcessing, setIsImageProcessing] = useState<boolean>(false);

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
      // Default insert at center-bottom: x=100, y=100, width=200, height=150
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
              "flex-1 text-xs font-medium py-1.5 px-1.5 rounded-md transition-all flex items-center justify-center gap-1",
              activeTab === "fields"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900",
            )}
          >
            <FileEdit className="size-3.5" />
            Forms ({fields.length})
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("text")}
            className={cn(
              "flex-1 text-xs font-medium py-1.5 px-1.5 rounded-md transition-all flex items-center justify-center gap-1",
              activeTab === "text"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900",
            )}
          >
            <Type className="size-3.5" />
            Text ({textSpans.length})
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("images")}
            className={cn(
              "flex-1 text-xs font-medium py-1.5 px-1.5 rounded-md transition-all flex items-center justify-center gap-1",
              activeTab === "images"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900",
            )}
          >
            <ImageIcon className="size-3.5" />
            Images ({images.length})
          </button>
        </div>

        {activeTab === "text" ? (
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
                        "p-2.5 rounded-lg border text-xs transition-all cursor-pointer",
                        isSelected
                          ? "border-sky-500 bg-sky-50/50 shadow-sm"
                          : "border-slate-200 bg-white hover:border-slate-300",
                      )}
                    >
                      <div className="flex items-start justify-between gap-1.5 mb-1.5">
                        <div className="flex items-center gap-1 font-mono text-[10px] text-slate-500 truncate">
                          <span>{span.span_id}</span>
                          <span>•</span>
                          <span>{span.font_name}</span>
                        </div>
                        {isEditable ? (
                          <Badge
                            variant="outline"
                            className="text-[9px] px-1 py-0 border-emerald-300 text-emerald-700 bg-emerald-50 flex items-center gap-0.5"
                          >
                            <CheckCircle2 className="size-2.5" />
                            Editable
                          </Badge>
                        ) : (
                          <Badge
                            variant="outline"
                            className="text-[9px] px-1 py-0 border-amber-300 text-amber-700 bg-amber-50 flex items-center gap-0.5"
                            title={span.refusal_reason || span.editability_code}
                          >
                            <AlertTriangle className="size-2.5" />
                            Refused
                          </Badge>
                        )}
                      </div>

                      <p className="text-slate-800 font-medium text-xs break-words line-clamp-2">
                        {span.text || <span className="italic text-slate-400">(empty)</span>}
                      </p>

                      {isSelected && (
                        <div className="mt-2.5 pt-2 border-t border-sky-200/60 space-y-2">
                          <Label className="text-[10px] text-sky-900 font-medium block">
                            Replace In-Stream Text:
                          </Label>
                          <Input
                            value={replacementText}
                            onChange={(e) => setReplacementText(e.target.value)}
                            className="h-7 text-xs bg-white border-sky-300 focus-visible:ring-sky-500"
                            placeholder="Enter replacement..."
                            disabled={!isEditable || isReplacing}
                          />
                          {!isEditable && (
                            <p className="text-[10px] text-amber-600 flex items-center gap-1">
                              <XCircle className="size-3 shrink-0" />
                              {span.refusal_reason || "Unsupported encoding or complex script."}
                            </p>
                          )}
                          <Button
                            size="sm"
                            onClick={handleApplyTextEdit}
                            disabled={!isEditable || isReplacing || replacementText === span.text}
                            className="w-full h-7 text-xs bg-sky-600 hover:bg-sky-700 text-white flex items-center justify-center gap-1"
                          >
                            <ArrowRight className="size-3" />
                            {isReplacing ? "Applying..." : "Apply Text Change"}
                          </Button>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        ) : activeTab === "images" ? (
          /* Image Objects List & Bounded Editor */
          <div className="space-y-3">
            <div className="flex items-center justify-between pb-1 border-b border-slate-100">
              <span className="text-xs font-semibold text-slate-800">
                Page Images ({images.length})
              </span>
              <Button
                size="sm"
                variant="outline"
                onClick={() => addImageInputRef.current?.click()}
                disabled={isImageProcessing}
                className="h-6 text-[11px] px-2 border-sky-300 text-sky-700 bg-sky-50 hover:bg-sky-100 flex items-center gap-1"
              >
                <Plus className="size-3" />
                Add Image
              </Button>
            </div>

            {/* Hidden File Inputs */}
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

            {images.length === 0 ? (
              <div className="text-center py-8 space-y-2 border border-dashed border-slate-200 rounded-lg p-4">
                <ImageIcon className="size-8 text-slate-300 mx-auto" />
                <p className="text-xs text-slate-400 italic">No image objects detected on this page.</p>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => addImageInputRef.current?.click()}
                  disabled={isImageProcessing}
                  className="h-7 text-xs border-sky-300 text-sky-700 bg-sky-50 hover:bg-sky-100"
                >
                  <Upload className="size-3 mr-1" />
                  Insert Image Here
                </Button>
              </div>
            ) : (
              <div className="space-y-2.5 pr-1 max-h-[480px] overflow-y-auto">
                {images.map((img) => {
                  const widthPt = Math.round(img.rect[2] - img.rect[0]);
                  const heightPt = Math.round(img.rect[3] - img.rect[1]);

                  return (
                    <div
                      key={img.image_id}
                      className="p-2.5 rounded-lg border border-slate-200 bg-slate-50/50 space-y-2 text-xs"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-mono font-medium text-[11px] text-slate-800">
                          /{img.resource_name}
                        </span>
                        <div className="flex items-center gap-1">
                          {img.is_shared && (
                            <Badge variant="outline" className="text-[9px] px-1 py-0 border-amber-300 text-amber-700 bg-amber-50">
                              Shared
                            </Badge>
                          )}
                          {img.is_nested_form && (
                            <Badge variant="outline" className="text-[9px] px-1 py-0 border-indigo-300 text-indigo-700 bg-indigo-50">
                              Form XObject
                            </Badge>
                          )}
                        </div>
                      </div>

                      <div className="grid grid-cols-2 gap-1 text-[11px] text-slate-600 bg-white p-2 rounded border border-slate-100">
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
