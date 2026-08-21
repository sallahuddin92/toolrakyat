"use client";

import { useId, useState } from "react";
import { type AcroFormField } from "@/lib/pdf/pdf-types";
import type { StarPdfTextSpan } from "@/lib/pdf/starpdf-types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { RotateCcw, FileEdit, Type, CheckCircle2, AlertTriangle, XCircle, ArrowRight } from "lucide-react";
import { cn } from "@/lib/utils";

interface PdfFormInspectorProps {
  fields: AcroFormField[];
  fieldValues: Record<string, string | boolean | string[]>;
  onFieldValueChange: (name: string, value: string | boolean | string[]) => void;
  onResetForm: () => void;
  isModified: boolean;
  textSpans?: StarPdfTextSpan[];
  onReplaceText?: (spanId: string, newText: string) => Promise<void>;
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
  className = "",
}: PdfFormInspectorProps) {
  const baseId = useId();
  const [activeTab, setActiveTab] = useState<"fields" | "text">(() =>
    fields.length > 0 ? "fields" : "text",
  );
  const [selectedSpanId, setSelectedSpanId] = useState<string | null>(null);
  const [replacementText, setReplacementText] = useState<string>("");
  const [isReplacing, setIsReplacing] = useState<boolean>(false);

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
              "flex-1 text-xs font-medium py-1.5 px-2 rounded-md transition-all flex items-center justify-center gap-1.5",
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
              "flex-1 text-xs font-medium py-1.5 px-2 rounded-md transition-all flex items-center justify-center gap-1.5",
              activeTab === "text"
                ? "bg-white text-slate-900 shadow-sm"
                : "text-slate-600 hover:text-slate-900",
            )}
          >
            <Type className="size-3.5" />
            Text ({textSpans.length})
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
                          : "border-slate-200/80 bg-slate-50/60 hover:border-slate-300",
                      )}
                    >
                      <div className="flex items-start justify-between gap-1 mb-1">
                        <span className="font-mono text-[10px] text-slate-500 truncate max-w-[130px]" title={span.font_name}>
                          {span.font_name} ({Math.round(span.font_size)}pt)
                        </span>
                        {isEditable ? (
                          <Badge className="bg-emerald-100 text-emerald-800 text-[9px] px-1.5 py-0 border-0 flex items-center gap-0.5">
                            <CheckCircle2 className="size-2.5" />
                            Editable
                          </Badge>
                        ) : (
                          <Badge variant="outline" className="bg-amber-50 text-amber-800 border-amber-200 text-[9px] px-1.5 py-0 flex items-center gap-0.5">
                            <AlertTriangle className="size-2.5" />
                            {span.editability_code === "UNSUPPORTED_COMPLEX_SCRIPT"
                              ? "Complex Script"
                              : span.editability_code === "UNSUPPORTED_FONT_ENCODING"
                              ? "Font Unmapped"
                              : "Refused"}
                          </Badge>
                        )}
                      </div>

                      <p className="font-medium text-slate-800 text-xs break-words line-clamp-2">
                        &ldquo;{span.text}&rdquo;
                      </p>

                      {isSelected && isEditable && (
                        <div className="mt-2.5 pt-2 border-t border-sky-200/60 space-y-2" onClick={(e) => e.stopPropagation()}>
                          <Label className="text-[10px] font-semibold text-sky-900">
                            Replacement Text:
                          </Label>
                          <Input
                            type="text"
                            value={replacementText}
                            onChange={(e) => setReplacementText(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") void handleApplyTextEdit();
                            }}
                            className="h-7 text-xs bg-white border-sky-300 focus:border-sky-500"
                            placeholder="Enter new text..."
                          />
                          <Button
                            type="button"
                            size="sm"
                            onClick={() => void handleApplyTextEdit()}
                            disabled={isReplacing || replacementText === span.text}
                            className="w-full h-7 text-xs bg-sky-600 hover:bg-sky-700 text-white gap-1"
                          >
                            {isReplacing ? "Replacing..." : "Apply Text Replacement"}
                            <ArrowRight className="size-3" />
                          </Button>
                        </div>
                      )}

                      {isSelected && !isEditable && (
                        <div className="mt-2 pt-2 border-t border-amber-200/60 text-[10px] text-amber-900 bg-amber-50/80 p-1.5 rounded">
                          <p className="font-semibold flex items-center gap-1">
                            <XCircle className="size-3 text-amber-700" />
                            Replacement Refused:
                          </p>
                          <p className="mt-0.5 text-amber-800 leading-snug">
                            {span.refusal_reason || "This font program does not support safe bidirectional Unicode glyph re-encoding."}
                          </p>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        ) : fields.length === 0 ? (
          <div className="space-y-4 text-center my-12">
            <div className="size-10 rounded-xl bg-slate-100 text-slate-400 flex items-center justify-center mx-auto">
              <FileEdit className="size-5" />
            </div>
            <div className="space-y-1">
              <h4 className="text-xs font-semibold text-slate-800">No AcroForm Fields</h4>
              <p className="text-[11px] text-slate-500 max-w-[200px] mx-auto leading-relaxed">
                Use the &ldquo;Text&rdquo; tab to inspect and replace native content-stream text.
              </p>
            </div>
          </div>
        ) : (
          /* Form Fields List */
          <div className="space-y-4 pr-1">
            {fields.map((field, index) => {
              const inputId = `${baseId}-${field.name}-${index}`;
            const currentValue = fieldValues[field.name] ?? field.value;

            return (
              <div
                key={field.name}
                className="space-y-1.5 p-3 rounded-xl bg-slate-50 border border-slate-200/80 transition-all hover:border-slate-300"
              >
                <div className="flex items-center justify-between">
                  <Label
                    htmlFor={inputId}
                    className="text-xs font-medium text-slate-800 truncate max-w-[180px]"
                    title={field.name}
                  >
                    {field.name}
                    {field.isRequired && <span className="text-red-500 ml-0.5">*</span>}
                  </Label>
                  <span className="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-slate-200/70 text-slate-600">
                    {field.type}
                  </span>
                </div>

                {/* Text Field */}
                {field.type === "text" && (
                  <Input
                    id={inputId}
                    type="text"
                    value={typeof currentValue === "string" ? currentValue : ""}
                    onChange={(e) => onFieldValueChange(field.name, e.target.value)}
                    disabled={field.isReadOnly}
                    className="h-8 text-xs bg-white"
                    placeholder="Enter text..."
                  />
                )}

                {/* Checkbox Field */}
                {field.type === "checkbox" && (
                  <div className="flex items-center gap-2 pt-1">
                    <Checkbox
                      id={inputId}
                      checked={Boolean(currentValue)}
                      onCheckedChange={(checked) => onFieldValueChange(field.name, Boolean(checked))}
                      disabled={field.isReadOnly}
                    />
                    <label htmlFor={inputId} className="text-xs text-slate-600 cursor-pointer select-none">
                      {Boolean(currentValue) ? "Checked" : "Unchecked"}
                    </label>
                  </div>
                )}

                {/* Radio Group Field */}
                {field.type === "radio" && field.options && (
                  <div className="space-y-1 pt-1">
                    {field.options.map((opt) => {
                      const radioId = `${inputId}-${opt}`;
                      const isSelected = currentValue === opt;
                      return (
                        <div key={opt} className="flex items-center gap-2">
                          <input
                            type="radio"
                            id={radioId}
                            name={field.name}
                            value={opt}
                            checked={isSelected}
                            onChange={() => onFieldValueChange(field.name, opt)}
                            disabled={field.isReadOnly}
                            className="size-3.5 text-sky-600 focus:ring-sky-500 border-slate-300"
                          />
                          <label htmlFor={radioId} className="text-xs text-slate-600 cursor-pointer select-none">
                            {opt}
                          </label>
                        </div>
                      );
                    })}
                  </div>
                )}

                {/* Dropdown Field */}
                {field.type === "dropdown" && field.options && (
                  <select
                    id={inputId}
                    value={typeof currentValue === "string" ? currentValue : ""}
                    onChange={(e) => onFieldValueChange(field.name, e.target.value)}
                    disabled={field.isReadOnly}
                    className="w-full h-8 rounded-md border border-slate-200 bg-white px-2 text-xs text-slate-800 focus:border-sky-500 focus:outline-none focus:ring-1 focus:ring-sky-500"
                  >
                    <option value="">(Select an option)</option>
                    {field.options.map((opt) => (
                      <option key={opt} value={opt}>
                        {opt}
                      </option>
                    ))}
                  </select>
                )}

                {/* Option List Field */}
                {field.type === "optionList" && field.options && (
                  <select
                    id={inputId}
                    multiple
                    value={Array.isArray(currentValue) ? currentValue : []}
                    onChange={(e) => {
                      const selected = Array.from(e.target.selectedOptions, (o) => o.value);
                      onFieldValueChange(field.name, selected);
                    }}
                    disabled={field.isReadOnly}
                    className="w-full h-20 rounded-md border border-slate-200 bg-white p-1 text-xs text-slate-800 focus:border-sky-500 focus:outline-none focus:ring-1 focus:ring-sky-500"
                  >
                    {field.options.map((opt) => (
                      <option key={opt} value={opt}>
                        {opt}
                      </option>
                    ))}
                  </select>
                )}

                {/* Unsupported Field */}
                {field.type === "unsupported" && (
                  <p className="text-[11px] text-slate-400 italic">Unsupported widget type</p>
                )}
              </div>
            );
          })}
        </div>
        )}
      </div>

      {/* Footer Reset Control */}
      <div className="pt-3 border-t border-slate-100 flex items-center justify-between">
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onResetForm}
          disabled={!isModified}
          className="text-xs text-slate-600 gap-1.5 w-full"
        >
          <RotateCcw className="size-3.5" />
          Reset Form to Original
        </Button>
      </div>
    </aside>
  );
}
